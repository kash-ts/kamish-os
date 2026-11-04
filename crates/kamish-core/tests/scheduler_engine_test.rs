use chrono::{Duration, TimeZone, Utc};
use kamish_core::audio::alert::set_audio_muted;
use kamish_core::database::{init_pool, run_migrations};
use kamish_core::scheduler::models::{CreateAlarmRequest, CreateReminderRequest};
use kamish_core::scheduler::repository::{
    create_alarm, create_reminder, get_alarm_by_id, get_reminder_by_id,
};
use kamish_core::scheduler::{SchedulerEngine, SchedulerEvent};

#[tokio::test]
async fn test_scheduler_firing_and_event_broadcast() {
    set_audio_muted(true);
    let pool = init_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    let scheduler = SchedulerEngine::new(pool.clone());
    let mut rx = scheduler.subscribe();

    // 1. Create a one-shot alarm
    let alarm_req = CreateAlarmRequest {
        time: "08:00".to_string(),
        recurrence: None, // one-shot
        label: Some("Doctor Appointment".to_string()),
        timezone: Some("UTC".to_string()),
        enabled: Some(true),
    };
    let alarm = create_alarm(&pool, &alarm_req).await.unwrap();

    // 2. Create a reminder
    let reminder_req = CreateReminderRequest {
        trigger_time: "2026-11-01T07:30:00Z".to_string(),
        text: "Take medicine".to_string(),
        timezone: Some("UTC".to_string()),
    };
    let reminder = create_reminder(&pool, &reminder_req).await.unwrap();

    // Load into scheduler at simulated 07:00
    let base_time = Utc.with_ymd_and_hms(2026, 11, 1, 7, 0, 0).unwrap();
    scheduler.reload_at(base_time).await;

    // Check status before firing
    let status_before = scheduler.get_status_summary().await;
    assert_eq!(status_before.active_alarms_count, 1);
    assert_eq!(status_before.pending_reminders_count, 1);

    // Simulate time at 07:15 (nothing due yet)
    scheduler
        .check_and_fire_due(Utc.with_ymd_and_hms(2026, 11, 1, 7, 15, 0).unwrap())
        .await;
    assert!(rx.try_recv().is_err());

    // Simulate time at 07:30 (reminder is due!)
    scheduler
        .check_and_fire_due(Utc.with_ymd_and_hms(2026, 11, 1, 7, 30, 0).unwrap())
        .await;

    // Verify reminder event received
    let event1 = rx.recv().await.unwrap();
    match event1 {
        SchedulerEvent::ReminderTriggered(r) => {
            assert_eq!(r.id, reminder.id);
            assert_eq!(r.text, "Take medicine");
        }
        _ => panic!("Expected ReminderTriggered"),
    }

    // Verify reminder status in SQLite is now 'completed'
    let reminder_db = get_reminder_by_id(&pool, &reminder.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reminder_db.status, "completed");

    // Simulate time at 08:00 (alarm is due!)
    scheduler
        .check_and_fire_due(Utc.with_ymd_and_hms(2026, 11, 1, 8, 0, 0).unwrap())
        .await;

    // Verify alarm event received
    let event2 = rx.recv().await.unwrap();
    match event2 {
        SchedulerEvent::AlarmTriggered(a) => {
            assert_eq!(a.id, alarm.id);
            assert_eq!(a.time, "08:00");
        }
        _ => panic!("Expected AlarmTriggered"),
    }

    // Since it was one-shot, verify SQLite enabled = 0
    let alarm_db = get_alarm_by_id(&pool, &alarm.id).await.unwrap().unwrap();
    assert!(
        !alarm_db.enabled,
        "One-shot alarm should be disabled after firing"
    );

    // Check status summary after all fired
    let status_after = scheduler.get_status_summary().await;
    assert_eq!(status_after.active_alarms_count, 0);
    assert_eq!(status_after.pending_reminders_count, 0);
}

#[tokio::test]
async fn test_recurring_alarm_remains_enabled() {
    set_audio_muted(true);
    let pool = init_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    let scheduler = SchedulerEngine::new(pool.clone());
    let mut rx = scheduler.subscribe();

    // Daily recurring alarm at 08:00
    let alarm_req = CreateAlarmRequest {
        time: "08:00".to_string(),
        recurrence: Some("daily".to_string()),
        label: Some("Daily Exercise".to_string()),
        timezone: Some("UTC".to_string()),
        enabled: Some(true),
    };
    let alarm = create_alarm(&pool, &alarm_req).await.unwrap();

    let base_time = Utc.with_ymd_and_hms(2026, 11, 1, 7, 0, 0).unwrap();
    scheduler.reload_at(base_time).await;

    // Fire on day 1 at 08:00
    scheduler
        .check_and_fire_due(Utc.with_ymd_and_hms(2026, 11, 1, 8, 0, 0).unwrap())
        .await;

    let event = rx.recv().await.unwrap();
    match event {
        SchedulerEvent::AlarmTriggered(a) => assert_eq!(a.id, alarm.id),
        _ => panic!("Expected AlarmTriggered"),
    }

    // Check alarm remains enabled in DB
    let alarm_db = get_alarm_by_id(&pool, &alarm.id).await.unwrap().unwrap();
    assert!(alarm_db.enabled, "Recurring alarm must remain enabled");

    // Active count in scheduler remains 1 (scheduled for tomorrow)
    let status = scheduler.get_status_summary().await;
    assert_eq!(status.active_alarms_count, 1);

    // Fire again on day 2 at 08:00
    scheduler
        .check_and_fire_due(Utc.with_ymd_and_hms(2026, 11, 2, 8, 0, 0).unwrap())
        .await;

    let event_day2 = rx.recv().await.unwrap();
    match event_day2 {
        SchedulerEvent::AlarmTriggered(a) => assert_eq!(a.id, alarm.id),
        _ => panic!("Expected AlarmTriggered on day 2"),
    }
}

#[tokio::test]
async fn test_recovery_after_reboot() {
    set_audio_muted(true);
    let pool = init_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    // 1. First "boot": create alarms and reminders
    let alarm_req = CreateAlarmRequest {
        time: "09:00".to_string(),
        recurrence: Some("weekdays".to_string()),
        label: Some("Work Alarm".to_string()),
        timezone: Some("UTC".to_string()),
        enabled: Some(true),
    };
    create_alarm(&pool, &alarm_req).await.unwrap();

    let reminder_req = CreateReminderRequest {
        trigger_time: (Utc::now() + Duration::hours(2)).to_rfc3339(),
        text: "Standup meeting".to_string(),
        timezone: Some("UTC".to_string()),
    };
    create_reminder(&pool, &reminder_req).await.unwrap();

    // 2. Simulate "reboot": instantiate a completely new SchedulerEngine with the same DB pool
    let new_scheduler = SchedulerEngine::new(pool.clone());
    new_scheduler.reload().await;

    let summary = new_scheduler.get_status_summary().await;
    assert_eq!(summary.active_alarms_count, 1);
    assert_eq!(summary.pending_reminders_count, 1);
}
