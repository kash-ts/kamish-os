import React, { useState, useEffect, useCallback } from 'react';
import {
  Play,
  Pause,
  Square,
  SkipForward,
  Heart,
  Volume2,
  VolumeX,
  Search,
  RefreshCw,
} from 'lucide-react';
import {
  getMusicCurrent,
  playMusic,
  pauseMusic,
  resumeMusic,
  stopMusic,
  skipMusic,
  likeMusic,
  searchMusic,
  getMusicFavorites,
  getMusicRecommendations,
  getRadioStations,
  getMusicHistory,
  getLavalinkStatus,
  updateLavalinkConfig,
  getYouTubeAuthStatus,
  beginYouTubeAuth,
  finishYouTubeAuth,
  logoutYouTubeAuth,
} from '../api';
import type {
  PlaybackState,
  Track,
  SearchResult,
  RadioStation,
  RecommendationItem,
  HistoryItem,
  LavalinkStatus,
  YouTubeAuthStatus,
  DeviceCodeChallenge,
} from '../types';

export const MusicPage: React.FC = () => {
  // Playback state
  const [playback, setPlayback] = useState<PlaybackState>({
    status: 'stopped',
    current_track: null,
    volume: 50,
    position_secs: 0,
    duration_secs: undefined,
  });
  const [isLiked, setIsLiked] = useState<boolean>(false);

  // Search state
  const [searchQuery, setSearchQuery] = useState('');
  const [searchSource, setSearchSource] = useState('all');
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);

  // Lists state
  const [favorites, setFavorites] = useState<Track[]>([]);
  const [recommendations, setRecommendations] = useState<RecommendationItem[]>([]);
  const [radioStations, setRadioStations] = useState<RadioStation[]>([]);
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [activeTab, setActiveTab] = useState<'radio' | 'recommendations' | 'favorites' | 'history'>('radio');
  const [isMuted, setIsMuted] = useState(false);
  const [prevVolume, setPrevVolume] = useState(50);

  // Lavalink Multi-Source Streaming state
  const [lavalinkStatus, setLavalinkStatus] = useState<LavalinkStatus | null>(null);
  const [showLavalinkConfig, setShowLavalinkConfig] = useState(false);
  const [yandexToken, setYandexToken] = useState('');
  const [isSavingLavalink, setIsSavingLavalink] = useState(false);
  const [lavalinkSaveMsg, setLavalinkSaveMsg] = useState('');

  // YouTube OAuth 2.0 Device Flow state
  const [ytStatus, setYtStatus] = useState<YouTubeAuthStatus | null>(null);
  const [showYtAuth, setShowYtAuth] = useState(false);
  const [ytClientId, setYtClientId] = useState('');
  const [ytClientSecret, setYtClientSecret] = useState('');
  const [ytChallenge, setYtChallenge] = useState<DeviceCodeChallenge | null>(null);
  const [isStartingYtAuth, setIsStartingYtAuth] = useState(false);
  const [isCheckingYtAuth, setIsCheckingYtAuth] = useState(false);
  const [ytAuthMsg, setYtAuthMsg] = useState('');

  const refreshYtStatus = useCallback(async () => {
    try {
      const st = await getYouTubeAuthStatus();
      setYtStatus(st);
    } catch {
      // offline / not configured
    }
  }, []);

  const refreshLavalink = useCallback(async () => {
    try {
      const st = await getLavalinkStatus();
      setLavalinkStatus(st);
    } catch {
      // Lavalink not responding or offline
    }
  }, []);

  const refreshPlayback = useCallback(async () => {
    try {
      const state = await getMusicCurrent();
      setPlayback(state);

      if (state.current_track?.id) {
        const favsRes = await getMusicFavorites();
        setFavorites(favsRes.favorites || []);
        const liked = (favsRes.favorites || []).some((f) => f.id === state.current_track?.id);
        setIsLiked(liked);
      }
    } catch (err) {
      console.warn('Failed to poll music playback status:', err);
    }
  }, []);

  const loadInitialData = useCallback(async () => {
    try {
      const [favsRes, recsRes, radioRes, histRes] = await Promise.all([
        getMusicFavorites().catch(() => ({ favorites: [] as Track[] })),
        getMusicRecommendations().catch(() => ({ recommendations: [] as RecommendationItem[] })),
        getRadioStations().catch(() => ({ stations: [] as RadioStation[] })),
        getMusicHistory().catch(() => ({ history: [] as HistoryItem[] })),
      ]);

      setFavorites(favsRes.favorites || []);
      setRecommendations(recsRes.recommendations || []);
      setRadioStations(radioRes.stations || []);
      setHistory(histRes.history || []);
    } catch (err) {
      console.warn('Failed to load music collections:', err);
    }
  }, []);

  useEffect(() => {
    refreshPlayback();
    loadInitialData();
    refreshLavalink();
    refreshYtStatus();

    const interval = setInterval(refreshPlayback, 3000);
    return () => clearInterval(interval);
  }, [refreshPlayback, loadInitialData, refreshLavalink, refreshYtStatus]);

  const handleSaveLavalinkConfig = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSavingLavalink(true);
    setLavalinkSaveMsg('');
    try {
      const res = await updateLavalinkConfig({
        yandex_music_token: yandexToken,
      });
      setLavalinkStatus(res.lavalink);
      setLavalinkSaveMsg('Настройки Lavalink успешно сохранены');
      setTimeout(() => setLavalinkSaveMsg(''), 4000);
    } catch (err) {
      console.error('Failed to save Lavalink config:', err);
      setLavalinkSaveMsg('Ошибка сохранения настроек');
    } finally {
      setIsSavingLavalink(false);
    }
  };

  const handleBeginYtAuth = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsStartingYtAuth(true);
    setYtAuthMsg('');
    try {
      const challenge = await beginYouTubeAuth(ytClientId.trim(), ytClientSecret.trim());
      setYtChallenge(challenge);
      setYtAuthMsg('Ссылка для авторизации готова. Перейдите по ссылке и подтвердите доступ.');
    } catch (err: any) {
      console.error('Failed to begin YouTube auth:', err);
      setYtAuthMsg(err?.message || 'Ошибка генерации ссылки');
    } finally {
      setIsStartingYtAuth(false);
    }
  };

  const handleFinishYtAuth = async () => {
    if (!ytChallenge) return;
    setIsCheckingYtAuth(true);
    setYtAuthMsg('');
    try {
      const res = await finishYouTubeAuth(ytChallenge.device_code);
      setYtStatus(res);
      setYtChallenge(null);
      setYtAuthMsg('Авторизация успешно завершена! Куки сгенерированы на сервере.');
      setTimeout(() => setYtAuthMsg(''), 5000);
    } catch (err: any) {
      console.error('Failed to finish YouTube auth:', err);
      setYtAuthMsg('Ожидание подтверждения... Нажмите «Разрешить» в окне Google и повторите.');
    } finally {
      setIsCheckingYtAuth(false);
    }
  };

  const handleLogoutYtAuth = async () => {
    try {
      await logoutYouTubeAuth();
      refreshYtStatus();
      setYtChallenge(null);
      setYtAuthMsg('Учётные данные и файл cookies.txt удалены');
      setTimeout(() => setYtAuthMsg(''), 4000);
    } catch (err) {
      console.error('Logout error:', err);
    }
  };

  const handlePlayPause = async () => {
    try {
      if (playback.status === 'playing') {
        await pauseMusic();
      } else if (playback.status === 'paused') {
        await resumeMusic();
      } else {
        await playMusic();
      }
      refreshPlayback();
    } catch (err) {
      console.error('Play/Pause error:', err);
    }
  };

  const handleStop = async () => {
    try {
      await stopMusic();
      refreshPlayback();
    } catch (err) {
      console.error('Stop error:', err);
    }
  };

  const handleSkip = async () => {
    try {
      await skipMusic();
      refreshPlayback();
    } catch (err) {
      console.error('Skip error:', err);
    }
  };

  const handleLike = async () => {
    if (!playback.current_track?.id) return;
    try {
      const nextLiked = !isLiked;
      await likeMusic({ track_id: playback.current_track.id, liked: nextLiked });
      setIsLiked(nextLiked);

      const favsRes = await getMusicFavorites();
      setFavorites(favsRes.favorites || []);
    } catch (err) {
      console.error('Like error:', err);
    }
  };

  const handleVolumeChange = async (newVol: number) => {
    const clamped = Math.min(85, Math.max(0, newVol));
    setPlayback((prev) => ({ ...prev, volume: clamped }));
    setIsMuted(clamped === 0);
  };

  const toggleMute = async () => {
    if (isMuted) {
      await handleVolumeChange(prevVolume > 0 ? prevVolume : 50);
      setIsMuted(false);
    } else {
      setPrevVolume(playback.volume);
      await handleVolumeChange(0);
      setIsMuted(true);
    }
  };

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!searchQuery.trim()) return;

    setIsSearching(true);
    try {
      const res = await searchMusic(searchQuery.trim(), searchSource);
      setSearchResults(res.results || []);
    } catch (err) {
      console.error('Search error:', err);
    } finally {
      setIsSearching(false);
    }
  };

  const handlePlayTrack = async (params: {
    track_id?: string;
    uri?: string;
    title?: string;
    artist?: string;
  }) => {
    try {
      await playMusic(params);
      refreshPlayback();
    } catch (err) {
      console.error('Play track error:', err);
    }
  };

  const formatSeconds = (sec?: number) => {
    if (sec === undefined || isNaN(sec)) return '--:--';
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s < 10 ? '0' : ''}${s}`;
  };

  return (
    <div className="space-y-6">
      {/* Header bar */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-5 rounded-xl bg-white border border-slate-200/80 shadow-xs">
        <div>
          <h2 className="text-base font-semibold text-slate-900 tracking-tight">
            Мультимедиа плеер
          </h2>
          <p className="text-xs text-slate-500 mt-0.5">
            Управление воспроизведением музыки, интернет-радио и медиатекой
          </p>
        </div>

        <button
          type="button"
          onClick={() => {
            refreshPlayback();
            loadInitialData();
          }}
          className="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-slate-700 bg-white border border-slate-200 rounded-lg hover:bg-slate-50 transition-colors cursor-pointer self-start sm:self-auto shadow-xs"
        >
          <RefreshCw className="w-3.5 h-3.5" />
          <span>Обновить</span>
        </button>
      </div>

      {/* Currently Playing Card */}
      <div className="bg-white border border-slate-200/80 rounded-xl p-6 shadow-xs text-slate-900">
        <div className="flex flex-col md:flex-row items-center justify-between gap-6">
          {/* Track Info */}
          <div className="overflow-hidden flex-1 w-full md:w-auto">
            <div className="flex items-center gap-2 mb-1.5">
              <span className="inline-flex items-center gap-1.5 text-xs font-medium text-slate-700">
                <span
                  className={`w-1.5 h-1.5 rounded-full ${
                    playback.status === 'playing'
                      ? 'bg-emerald-500'
                      : playback.status === 'paused'
                      ? 'bg-amber-500'
                      : 'bg-slate-400'
                  }`}
                />
                {playback.status === 'playing' ? 'Воспроизведение' : playback.status === 'paused' ? 'Пауза' : 'Остановлен'}
              </span>

              {playback.current_track?.source && (
                <span className="text-[11px] px-2 py-0.5 rounded-md bg-slate-100 text-slate-600 font-mono">
                  {playback.current_track.source}
                </span>
              )}
            </div>

            <h3 className="text-base font-bold text-slate-900 truncate">
              {playback.current_track?.title || 'Нет активного трека'}
            </h3>
            <p className="text-slate-500 text-xs truncate mt-0.5">
              {playback.current_track?.artist || 'Плеер готов к приему команд'}
            </p>

            {/* Progress bar */}
            <div className="mt-3 flex items-center gap-3 text-xs text-slate-400">
              <span className="font-mono">{formatSeconds(playback.position_secs)}</span>
              <div className="flex-1 bg-slate-100 rounded-full h-1.5 overflow-hidden">
                <div
                  className="bg-slate-800 h-full rounded-full transition-all duration-300"
                  style={{
                    width: playback.duration_secs
                      ? `${Math.min(100, (playback.position_secs / playback.duration_secs) * 100)}%`
                      : playback.status === 'playing'
                      ? '100%'
                      : '0%',
                  }}
                />
              </div>
              <span className="font-mono">
                {playback.duration_secs ? formatSeconds(playback.duration_secs) : '--:--'}
              </span>
            </div>
          </div>

          {/* Controls */}
          <div className="flex items-center gap-2 w-full md:w-auto justify-center">
            <button
              type="button"
              onClick={handleLike}
              disabled={!playback.current_track}
              className={`p-2.5 rounded-lg border border-slate-200 transition-colors cursor-pointer disabled:opacity-40 ${
                isLiked ? 'text-rose-600 bg-rose-50' : 'text-slate-600 hover:bg-slate-50'
              }`}
              title={isLiked ? 'Убрать из избранного' : 'В избранное'}
            >
              <Heart className={`w-4 h-4 ${isLiked ? 'fill-current' : ''}`} />
            </button>

            <button
              type="button"
              onClick={handleStop}
              className="p-2.5 rounded-lg border border-slate-200 text-slate-700 hover:bg-slate-50 transition-colors cursor-pointer"
              title="Стоп"
            >
              <Square className="w-4 h-4" />
            </button>

            <button
              type="button"
              onClick={handlePlayPause}
              className="p-3 rounded-lg bg-slate-900 hover:bg-slate-800 text-white transition-colors cursor-pointer shadow-xs"
              title={playback.status === 'playing' ? 'Пауза' : 'Воспроизведение'}
            >
              {playback.status === 'playing' ? (
                <Pause className="w-5 h-5 fill-current" />
              ) : (
                <Play className="w-5 h-5 fill-current" />
              )}
            </button>

            <button
              type="button"
              onClick={handleSkip}
              className="p-2.5 rounded-lg border border-slate-200 text-slate-700 hover:bg-slate-50 transition-colors cursor-pointer"
              title="Следующий трек"
            >
              <SkipForward className="w-4 h-4" />
            </button>

            {/* Volume slider */}
            <div className="flex items-center gap-2 ml-2 pl-3 border-l border-slate-200">
              <button
                type="button"
                onClick={toggleMute}
                className="text-slate-500 hover:text-slate-900 transition-colors cursor-pointer"
              >
                {isMuted || playback.volume === 0 ? (
                  <VolumeX className="w-4 h-4" />
                ) : (
                  <Volume2 className="w-4 h-4" />
                )}
              </button>
              <input
                type="range"
                min={0}
                max={85}
                value={isMuted ? 0 : playback.volume}
                onChange={(e) => handleVolumeChange(Number(e.target.value))}
                className="w-20 accent-slate-900 cursor-pointer"
                title={`Громкость: ${playback.volume}% (макс 85%)`}
              />
            </div>
          </div>
        </div>
      </div>

      {/* Lavalink Multi-Source Status & Quick Config */}
      <div className="bg-white rounded-xl p-4 border border-slate-200/80 shadow-xs flex flex-col gap-3">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <span
              className={`w-2.5 h-2.5 rounded-full ${
                lavalinkStatus?.connected ? 'bg-emerald-500' : 'bg-slate-300'
              }`}
            />
            <span className="text-xs font-semibold text-slate-900">
              Музыкальный сервер Lavalink v4
            </span>
            <span className="text-[11px] text-slate-500">
              {lavalinkStatus?.connected
                ? `(Активен: ${lavalinkStatus.host}:${lavalinkStatus.port})`
                : '(Режим прямого воспроизведения MPV)'}
            </span>
          </div>

          <div className="flex items-center gap-2">
            <span className="inline-flex items-center px-2 py-0.5 rounded text-[10px] font-medium bg-red-50 text-red-700 border border-red-200">
              YouTube
            </span>
            <span
              className={`inline-flex items-center px-2 py-0.5 rounded text-[10px] font-medium border ${
                lavalinkStatus?.yandex_music_configured
                  ? 'bg-amber-50 text-amber-800 border-amber-200'
                  : 'bg-slate-50 text-slate-600 border-slate-200'
              }`}
            >
              Яндекс Музыка {lavalinkStatus?.yandex_music_configured ? '✓' : '(нужен токен)'}
            </span>
            <span className="inline-flex items-center px-2 py-0.5 rounded text-[10px] font-medium bg-orange-50 text-orange-700 border border-orange-200">
              SoundCloud
            </span>

            <button
              type="button"
              onClick={() => setShowLavalinkConfig(!showLavalinkConfig)}
              className="text-xs text-slate-600 hover:text-slate-900 underline ml-2 cursor-pointer"
            >
              {showLavalinkConfig ? 'Скрыть настройки' : 'Настроить'}
            </button>
          </div>
        </div>

        {showLavalinkConfig && (
          <form
            onSubmit={handleSaveLavalinkConfig}
            className="pt-3 border-t border-slate-100 flex flex-col sm:flex-row gap-3 items-end"
          >
            <div className="flex-1 w-full">
              <label className="block text-[11px] font-medium text-slate-600 mb-1">
                OAuth токен Яндекс Музыки (для воспроизведения полных треков по подписке Яндекс Плюс)
              </label>
              <input
                type="password"
                value={yandexToken}
                onChange={(e) => setYandexToken(e.target.value)}
                placeholder={
                  lavalinkStatus?.yandex_music_configured
                    ? '•••••••••••••••• (токен сохранён)'
                    : 'Вставьте OAuth-токен аккаунта Яндекс'
                }
                className="w-full px-3 py-1.5 bg-white border border-slate-200 rounded-lg text-xs text-slate-900 focus:outline-none focus:ring-1 focus:ring-slate-800"
              />
            </div>
            <button
              type="submit"
              disabled={isSavingLavalink || !yandexToken.trim()}
              className="px-4 py-1.5 bg-slate-900 hover:bg-slate-800 text-white rounded-lg text-xs font-medium transition-colors disabled:opacity-50 cursor-pointer shadow-xs whitespace-nowrap"
            >
              {isSavingLavalink ? 'Сохранение...' : 'Сохранить токен'}
            </button>
            {lavalinkSaveMsg && (
              <span className="text-xs text-emerald-600 self-center">{lavalinkSaveMsg}</span>
            )}
          </form>
        )}
      </div>

      {/* YouTube OAuth Device Flow (One-Click Link Authorization) */}
      <div className="bg-white rounded-xl p-4 border border-slate-200/80 shadow-xs flex flex-col gap-3">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <span
              className={`w-2.5 h-2.5 rounded-full ${
                ytStatus?.authorized && ytStatus?.cookies_ready
                  ? 'bg-emerald-500'
                  : 'bg-amber-400'
              }`}
            />
            <span className="text-xs font-semibold text-slate-900">
              Авторизация YouTube / Google
            </span>
            <span className="text-[11px] text-slate-500">
              {ytStatus?.authorized && ytStatus?.cookies_ready
                ? '✓ Авторизован (файл cookies.txt активен)'
                : '(Требуется для обхода блокировок Google и бот-проверок)'}
            </span>
          </div>

          <div className="flex items-center gap-2">
            {ytStatus?.authorized ? (
              <button
                type="button"
                onClick={handleLogoutYtAuth}
                className="text-xs text-red-600 hover:text-red-800 underline cursor-pointer"
              >
                Выйти из Google
              </button>
            ) : (
              <button
                type="button"
                onClick={() => setShowYtAuth(!showYtAuth)}
                className="text-xs text-slate-600 hover:text-slate-900 underline cursor-pointer"
              >
                {showYtAuth ? 'Скрыть' : 'Авторизоваться ссылкой'}
              </button>
            )}
          </div>
        </div>

        {showYtAuth && !ytStatus?.authorized && (
          <div className="pt-3 border-t border-slate-100 flex flex-col gap-3">
            {!ytChallenge ? (
              <form onSubmit={handleBeginYtAuth} className="flex flex-col sm:flex-row gap-3 items-end">
                <div className="flex-1 w-full">
                  <label className="block text-[11px] font-medium text-slate-600 mb-1">
                    Google OAuth Client ID (тип: TV and Limited Input devices)
                  </label>
                  <input
                    type="text"
                    value={ytClientId}
                    onChange={(e) => setYtClientId(e.target.value)}
                    placeholder="Например: 123456789-abc.apps.googleusercontent.com"
                    className="w-full px-3 py-1.5 bg-white border border-slate-200 rounded-lg text-xs text-slate-900 focus:outline-none focus:ring-1 focus:ring-slate-800"
                    required
                  />
                </div>
                <div className="flex-1 w-full">
                  <label className="block text-[11px] font-medium text-slate-600 mb-1">
                    Client Secret
                  </label>
                  <input
                    type="password"
                    value={ytClientSecret}
                    onChange={(e) => setYtClientSecret(e.target.value)}
                    placeholder="GOCSPX-..."
                    className="w-full px-3 py-1.5 bg-white border border-slate-200 rounded-lg text-xs text-slate-900 focus:outline-none focus:ring-1 focus:ring-slate-800"
                    required
                  />
                </div>
                <button
                  type="submit"
                  disabled={isStartingYtAuth || !ytClientId.trim() || !ytClientSecret.trim()}
                  className="px-4 py-1.5 bg-slate-900 hover:bg-slate-800 text-white rounded-lg text-xs font-medium transition-colors disabled:opacity-50 cursor-pointer shadow-xs whitespace-nowrap"
                >
                  {isStartingYtAuth ? 'Генерация ссылки...' : 'Сгенерировать ссылку'}
                </button>
              </form>
            ) : (
              <div className="p-4 bg-slate-50 border border-slate-200 rounded-xl space-y-3">
                <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3">
                  <div>
                    <h5 className="text-xs font-semibold text-slate-900">
                      Шаг 1: Подтвердите доступ в Google
                    </h5>
                    <p className="text-[11px] text-slate-600 mt-0.5">
                      Код авторизации:{' '}
                      <span className="font-mono font-bold text-sm text-slate-900 px-2 py-0.5 bg-white border border-slate-200 rounded">
                        {ytChallenge.user_code}
                      </span>
                    </p>
                  </div>

                  <a
                    href={`${ytChallenge.verification_url}?user_code=${encodeURIComponent(
                      ytChallenge.user_code
                    )}`}
                    target="_blank"
                    rel="noreferrer"
                    className="inline-flex items-center gap-1.5 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-xs font-medium transition-colors cursor-pointer shadow-xs"
                  >
                    Перейти и подтвердить в один клик ↗
                  </a>
                </div>

                <div className="pt-2 border-t border-slate-200 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3">
                  <p className="text-[11px] text-slate-500">
                    Шаг 2: После нажатия кнопки «Разрешить» в Google, нажмите кнопку справа:
                  </p>

                  <button
                    type="button"
                    onClick={handleFinishYtAuth}
                    disabled={isCheckingYtAuth}
                    className="px-4 py-1.5 bg-slate-900 hover:bg-slate-800 text-white rounded-lg text-xs font-medium transition-colors disabled:opacity-50 cursor-pointer shadow-xs"
                  >
                    {isCheckingYtAuth ? 'Проверка...' : 'Завершить вход и создать куки'}
                  </button>
                </div>
              </div>
            )}

            {ytAuthMsg && <p className="text-xs text-slate-600">{ytAuthMsg}</p>}
          </div>
        )}
      </div>

      {/* Search Bar */}
      <div className="bg-white rounded-xl p-4 border border-slate-200/80 shadow-xs">
        <form onSubmit={handleSearch} className="flex flex-col sm:flex-row gap-3">
          <div className="relative flex-1">
            <Search className="w-4 h-4 absolute left-3 top-3 text-slate-400" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Поиск музыки, треков, радиостанций..."
              className="w-full pl-9 pr-4 py-2 bg-white border border-slate-200 rounded-lg text-xs text-slate-900 focus:outline-none focus:ring-2 focus:ring-slate-900/10 focus:border-slate-800"
            />
          </div>

          <div className="flex gap-2">
            <select
              value={searchSource}
              onChange={(e) => setSearchSource(e.target.value)}
              className="bg-white border border-slate-200 rounded-lg px-3 py-2 text-xs text-slate-900"
            >
              <option value="all">Все источники</option>
              <option value="youtube">YouTube Music</option>
              <option value="yandex">Яндекс Музыка (Lavalink)</option>
              <option value="soundcloud">SoundCloud (Lavalink)</option>
              <option value="radio">Интернет-радио</option>
              <option value="local">Локальная медиатека</option>
            </select>

            <button
              type="submit"
              disabled={isSearching || !searchQuery.trim()}
              className="px-4 py-2 bg-slate-900 hover:bg-slate-800 text-white rounded-lg text-xs font-medium transition-colors disabled:opacity-50 cursor-pointer shadow-xs"
            >
              {isSearching ? 'Поиск...' : 'Найти'}
            </button>
          </div>
        </form>

        {/* Search Results */}
        {searchResults.length > 0 && (
          <div className="mt-4 pt-4 border-t border-slate-100">
            <h4 className="text-xs font-semibold text-slate-700 mb-2">
              Результаты поиска ({searchResults.length})
            </h4>
            <div className="divide-y divide-slate-100 max-h-64 overflow-y-auto">
              {searchResults.map((res, idx) => (
                <div
                  key={idx}
                  className="py-2.5 flex items-center justify-between"
                >
                  <div className="overflow-hidden flex-1">
                    <p className="text-xs font-medium text-slate-900 truncate">{res.title}</p>
                    <p className="text-[11px] text-slate-500 truncate">{res.artist}</p>
                  </div>
                  <button
                    type="button"
                    onClick={() =>
                      handlePlayTrack({
                        track_id: res.id,
                        uri: res.uri,
                        title: res.title,
                        artist: res.artist,
                      })
                    }
                    className="p-1.5 text-slate-600 hover:text-slate-900 hover:bg-slate-100 rounded-lg transition-colors ml-2 shrink-0 cursor-pointer"
                    title="Воспроизвести"
                  >
                    <Play className="w-3.5 h-3.5 fill-current" />
                  </button>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* Tabs navigation */}
      <div className="flex items-center gap-1.5 border-b border-slate-200 pb-2 overflow-x-auto">
        {[
          { id: 'radio' as const, label: `Интернет-радио (${radioStations.length})` },
          { id: 'recommendations' as const, label: `Рекомендации (${recommendations.length})` },
          { id: 'favorites' as const, label: `Избранное (${favorites.length})` },
          { id: 'history' as const, label: `История (${history.length})` },
        ].map((tab) => (
          <button
            key={tab.id}
            type="button"
            onClick={() => setActiveTab(tab.id)}
            className={`px-3.5 py-1.5 rounded-lg text-xs font-medium transition-colors cursor-pointer ${
              activeTab === tab.id
                ? 'bg-slate-900 text-white shadow-xs'
                : 'text-slate-600 hover:bg-slate-100 hover:text-slate-900'
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Tab Panels */}
      {/* 1. Radio Stations */}
      {activeTab === 'radio' && (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
          {radioStations.map((station) => (
            <div
              key={station.id}
              className="bg-white rounded-xl p-4 border border-slate-200/80 shadow-xs flex items-center justify-between"
            >
              <div className="overflow-hidden flex-1">
                <span className="text-[10px] font-mono text-slate-400 uppercase block mb-0.5">
                  {station.genre}
                </span>
                <h4 className="text-xs font-semibold text-slate-900 truncate">{station.name}</h4>
                <p className="text-[11px] text-slate-400 truncate mt-0.5">{station.stream_uri}</p>
              </div>

              <button
                type="button"
                onClick={() =>
                  handlePlayTrack({
                    track_id: station.id,
                    uri: station.stream_uri,
                    title: station.name,
                    artist: station.genre,
                  })
                }
                className="w-8 h-8 rounded-lg bg-slate-100 hover:bg-slate-900 hover:text-white text-slate-700 flex items-center justify-center transition-colors shrink-0 ml-3 cursor-pointer"
                title="Слушать радио"
              >
                <Play className="w-3.5 h-3.5 fill-current" />
              </button>
            </div>
          ))}
        </div>
      )}

      {/* 2. Recommendations */}
      {activeTab === 'recommendations' && (
        <div className="bg-white rounded-xl border border-slate-200/80 shadow-xs p-4">
          {recommendations.length === 0 ? (
            <div className="text-center py-8 text-xs text-slate-500">
              Рекомендации формируются на основе локальной истории воспроизведения.
            </div>
          ) : (
            <div className="divide-y divide-slate-100">
              {recommendations.map((rec, i) => (
                <div
                  key={i}
                  className="py-3 flex items-center justify-between gap-4 first:pt-0 last:pb-0"
                >
                  <div className="overflow-hidden flex-1">
                    <h4 className="text-xs font-semibold text-slate-900 truncate">{rec.track.title}</h4>
                    <p className="text-xs text-slate-500 truncate mt-0.5">
                      {rec.track.artist} • <span className="text-slate-400">{rec.reason}</span>
                    </p>
                  </div>
                  <button
                    type="button"
                    onClick={() =>
                      handlePlayTrack({
                        track_id: rec.track.id,
                        title: rec.track.title,
                        artist: rec.track.artist,
                        uri: rec.track.uri,
                      })
                    }
                    className="px-3 py-1.5 rounded-lg bg-slate-900 hover:bg-slate-800 text-white text-xs font-medium flex items-center gap-1.5 transition-colors cursor-pointer shadow-xs"
                  >
                    <Play className="w-3 h-3 fill-current" />
                    <span>Слушать</span>
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* 3. Favorites */}
      {activeTab === 'favorites' && (
        <div className="bg-white rounded-xl border border-slate-200/80 shadow-xs p-4">
          {favorites.length === 0 ? (
            <div className="text-center py-8 text-xs text-slate-500">
              Список избранного пуст. Добавляйте треки во время воспроизведения.
            </div>
          ) : (
            <div className="divide-y divide-slate-100">
              {favorites.map((track) => (
                <div
                  key={track.id}
                  className="py-3 flex items-center justify-between gap-4 first:pt-0 last:pb-0"
                >
                  <div className="overflow-hidden flex-1">
                    <h4 className="text-xs font-semibold text-slate-900 truncate">{track.title}</h4>
                    <p className="text-xs text-slate-500 truncate mt-0.5">{track.artist}</p>
                  </div>
                  <button
                    type="button"
                    onClick={() =>
                      handlePlayTrack({
                        track_id: track.id,
                        uri: track.uri,
                        title: track.title,
                        artist: track.artist,
                      })
                    }
                    className="p-2 rounded-lg bg-slate-100 hover:bg-slate-900 hover:text-white text-slate-700 transition-colors cursor-pointer"
                    title="Воспроизвести"
                  >
                    <Play className="w-3.5 h-3.5 fill-current" />
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* 4. History */}
      {activeTab === 'history' && (
        <div className="bg-white rounded-xl border border-slate-200/80 shadow-xs p-4">
          {history.length === 0 ? (
            <div className="text-center py-8 text-xs text-slate-500">
              История воспроизведения пока пуста.
            </div>
          ) : (
            <div className="divide-y divide-slate-100">
              {history.map((item, i) => (
                <div
                  key={i}
                  className="py-2.5 flex items-center justify-between gap-3 text-xs first:pt-0 last:pb-0"
                >
                  <div className="overflow-hidden flex-1">
                    <p className="font-medium text-slate-900 truncate">{item.track?.title || item.track_id}</p>
                    <p className="text-slate-400 truncate text-[11px] mt-0.5">{item.track?.artist || 'Локальный трек'}</p>
                  </div>
                  <span className="text-[11px] font-mono text-slate-400 shrink-0">
                    {new Date(item.played_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
};
