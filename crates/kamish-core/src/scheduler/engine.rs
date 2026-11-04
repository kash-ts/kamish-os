//! Background Scheduler Engine.
//!
//! Evaluates active alarms and pending reminders, triggers alert tones,
//! updates persistence states, and maintains resilience to system clock changes.

use super::models::{Alarm, Reminder, SchedulerStatusSummary};
use super::recurrence::{calculate_next_alarm_trigger, parse_reminder_trigger_time};
use super::repository::{list_alarms, list_reminders, set_alarm_enabled, update_reminder_status};
use crate::audio::alert::{play_alert_tone, AlertType};
use crate::database::DbPool;
use chrono::{DateTime, Utc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, Notify, RwLock};
use tokio::time::{interval, Duration, Instant};
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct ScheduledAlarm {
    pub alarm: Alarm,
    pub next_trigger: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ScheduledReminder {
    pub reminder: Reminder,
    pub trigger_time: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum SchedulerEvent {
    AlarmTriggered(Alarm),
    ReminderTriggered(Reminder),
}

#[derive(Default)]
struct EngineState {
    alarms: Vec<ScheduledAlarm>,
    reminders: Vec<ScheduledReminder>,
}

pub struct SchedulerEngine {
    pool: DbPool,
    state: RwLock<EngineState>,
    running: AtomicBool,
    notify_change: Notify,
    event_tx: broadcast::Sender<SchedulerEvent>,
}

pub type SchedulerHandle = Arc<SchedulerEngine>;

impl SchedulerEngine {
    /// Create a new Scheduler engine.
    pub fn new(pool: DbPool) -> SchedulerHandle {
        let (event_tx, _) = broadcast::channel(32);
        Arc::new(Self {
            pool,
            state: RwLock::new(EngineState::default()),
            running: AtomicBool::new(false),
            notify_change: Notify::new(),
            event_tx,
        })
    }

    /// Subscribe to trigger events (alarms and reminders).
    pub fn subscribe(&self) -> broadcast::Receiver<SchedulerEvent> {
        self.event_tx.subscribe()
    }

    /// Notify the engine that alarms or reminders were modified.
    pub fn notify_modified(&self) {
        self.notify_change.notify_one();
    }

    /// Reload all active alarms and pending reminders from SQLite using current time.
    pub async fn reload(&self) {
        self.reload_at(Utc::now()).await;
    }

    /// Reload all active alarms and pending reminders from SQLite using a given reference time.
    pub async fn reload_at(&self, now: DateTime<Utc>) {
        // 1. Load active alarms
        let alarms_res = list_alarms(&self.pool).await;
        let mut scheduled_alarms = Vec::new();
        match alarms_res {
            Ok(all_alarms) => {
                for alarm in all_alarms.into_iter().filter(|a| a.enabled) {
                    match calculate_next_alarm_trigger(
                        &alarm.time,
                        alarm.recurrence.as_deref(),
                        &alarm.timezone,
                        now,
                    ) {
                        Ok(next) => {
                            debug!(
                                "Loaded alarm '{}' ({}): next trigger at {} UTC",
                                alarm.id, alarm.time, next
                            );
                            scheduled_alarms.push(ScheduledAlarm {
                                alarm,
                                next_trigger: next,
                            });
                        }
                        Err(e) => {
                            error!("Failed to schedule alarm '{}': {}", alarm.id, e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to reload alarms from DB: {}", e);
            }
        }

        // 2. Load pending reminders
        let reminders_res = list_reminders(&self.pool, Some("pending")).await;
        let mut scheduled_reminders = Vec::new();
        match reminders_res {
            Ok(reminders) => {
                for reminder in reminders {
                    match parse_reminder_trigger_time(&reminder.trigger_time, &reminder.timezone) {
                        Ok(dt) => {
                            scheduled_reminders.push(ScheduledReminder {
                                reminder,
                                trigger_time: dt,
                            });
                        }
                        Err(e) => {
                            error!(
                                "Failed to parse trigger_time for reminder '{}': {}",
                                reminder.id, e
                            );
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to reload reminders from DB: {}", e);
            }
        }

        let mut st = self.state.write().await;
        st.alarms = scheduled_alarms;
        st.reminders = scheduled_reminders;
        debug!(
            "Scheduler reloaded: {} active alarms, {} pending reminders",
            st.alarms.len(),
            st.reminders.len()
        );
    }

    /// Re-evaluates next trigger times for all alarms (e.g. after NTP or clock jump).
    async fn recalculate_all_alarms(&self, now: DateTime<Utc>) {
        let mut st = self.state.write().await;
        for item in &mut st.alarms {
            if let Ok(next) = calculate_next_alarm_trigger(
                &item.alarm.time,
                item.alarm.recurrence.as_deref(),
                &item.alarm.timezone,
                now,
            ) {
                item.next_trigger = next;
            }
        }
    }

    /// Check and fire due events at current time.
    pub async fn check_and_fire_due(&self, now: DateTime<Utc>) {
        let mut triggered_alarms = Vec::new();
        let mut triggered_reminders = Vec::new();

        {
            let mut st = self.state.write().await;

            // Check alarms
            let mut remaining_alarms = Vec::new();
            for mut item in st.alarms.drain(..) {
                if item.next_trigger <= now {
                    triggered_alarms.push(item.alarm.clone());

                    // If recurring, advance to next future occurrence
                    if item.alarm.recurrence.is_some() {
                        if let Ok(next) = calculate_next_alarm_trigger(
                            &item.alarm.time,
                            item.alarm.recurrence.as_deref(),
                            &item.alarm.timezone,
                            now,
                        ) {
                            item.next_trigger = next;
                            remaining_alarms.push(item);
                        }
                    }
                    // If one-shot, do not retain in active schedule
                } else {
                    remaining_alarms.push(item);
                }
            }
            st.alarms = remaining_alarms;

            // Check reminders
            let mut remaining_reminders = Vec::new();
            for item in st.reminders.drain(..) {
                if item.trigger_time <= now {
                    triggered_reminders.push(item.reminder);
                } else {
                    remaining_reminders.push(item);
                }
            }
            st.reminders = remaining_reminders;
        }

        // Process triggered alarms
        for alarm in triggered_alarms {
            info!(
                "ALARM TRIGGERED: id={}, time={}, label={:?}",
                alarm.id, alarm.time, alarm.label
            );

            // Execute local tone
            play_alert_tone(AlertType::Alarm).await;

            // Broadcast event
            let _ = self
                .event_tx
                .send(SchedulerEvent::AlarmTriggered(alarm.clone()));

            // If one-shot alarm, update DB to disabled
            if alarm.recurrence.is_none() {
                if let Err(e) = set_alarm_enabled(&self.pool, &alarm.id, false).await {
                    error!(
                        "Failed to disable fired one-shot alarm '{}': {}",
                        alarm.id, e
                    );
                }
            }
        }

        // Process triggered reminders
        for reminder in triggered_reminders {
            info!(
                "REMINDER TRIGGERED: id={}, text={}",
                reminder.id, reminder.text
            );

            // Execute local tone
            play_alert_tone(AlertType::Reminder).await;

            // Broadcast event
            let _ = self
                .event_tx
                .send(SchedulerEvent::ReminderTriggered(reminder.clone()));

            // Update DB to completed
            if let Err(e) = update_reminder_status(&self.pool, &reminder.id, "completed").await {
                error!(
                    "Failed to mark fired reminder '{}' as completed: {}",
                    reminder.id, e
                );
            }
        }
    }

    /// Get current runtime summary.
    pub async fn get_status_summary(&self) -> SchedulerStatusSummary {
        let st = self.state.read().await;
        SchedulerStatusSummary {
            active_alarms_count: st.alarms.len(),
            pending_reminders_count: st.reminders.len(),
            running: self.running.load(Ordering::Relaxed),
        }
    }

    /// Background runner loop.
    pub async fn run(self: Arc<Self>) {
        info!("Scheduler background loop started");
        self.running.store(true, Ordering::Relaxed);

        // Initial recovery from database
        self.reload().await;

        let mut ticker = interval(Duration::from_secs(1));
        let mut last_mono_tick = Instant::now();
        let mut last_wall_tick = Utc::now();

        while self.running.load(Ordering::Relaxed) {
            tokio::select! {
                _ = ticker.tick() => {
                    let now_mono = Instant::now();
                    let now_wall = Utc::now();

                    // Detect system time discontinuity (e.g. NTP sync, time shift > 3s)
                    let mono_delta = now_mono.duration_since(last_mono_tick).as_secs_f64();
                    let wall_delta = (now_wall - last_wall_tick).num_milliseconds() as f64 / 1000.0;

                    if (wall_delta - mono_delta).abs() > 3.0 {
                        warn!(
                            "System clock shift detected (wall delta: {:.2}s, mono delta: {:.2}s). Re-evaluating schedule.",
                            wall_delta, mono_delta
                        );
                        self.recalculate_all_alarms(now_wall).await;
                    }

                    last_mono_tick = now_mono;
                    last_wall_tick = now_wall;

                    self.check_and_fire_due(now_wall).await;
                }
                _ = self.notify_change.notified() => {
                    debug!("Scheduler change notification received, reloading state");
                    self.reload().await;
                }
            }
        }

        info!("Scheduler background loop stopped");
    }

    /// Stop the scheduler background loop.
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}
