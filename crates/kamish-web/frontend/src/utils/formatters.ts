export function formatBytes(bytes: number, decimals = 1): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
}

export function formatUptime(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSecs = seconds % 60;
  if (minutes < 60) {
    return `${minutes}m ${remainingSecs}s`;
  }
  const hours = Math.floor(minutes / 60);
  const remainingMins = minutes % 60;
  if (hours < 24) {
    return `${hours}h ${remainingMins}m`;
  }
  const days = Math.floor(hours / 24);
  const remainingHours = hours % 24;
  return `${days}d ${remainingHours}h`;
}

export function formatDateTime(isoString: string): string {
  try {
    const d = new Date(isoString);
    if (isNaN(d.getTime())) return isoString;
    return d.toLocaleString([], {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return isoString;
  }
}

export function formatRelativeTime(isoString: string): string {
  try {
    const date = new Date(isoString);
    if (isNaN(date.getTime())) return '';
    const now = new Date();
    const diffSec = Math.floor((date.getTime() - now.getTime()) / 1000);

    if (Math.abs(diffSec) < 60) {
      return diffSec >= 0 ? 'just now' : 'moments ago';
    }

    const diffMin = Math.floor(diffSec / 60);
    if (Math.abs(diffMin) < 60) {
      return diffMin > 0 ? `in ${diffMin} min` : `${Math.abs(diffMin)} min ago`;
    }

    const diffHours = Math.floor(diffMin / 60);
    if (Math.abs(diffHours) < 24) {
      return diffHours > 0 ? `in ${diffHours} hr` : `${Math.abs(diffHours)} hr ago`;
    }

    const diffDays = Math.floor(diffHours / 24);
    return diffDays > 0 ? `in ${diffDays} day(s)` : `${Math.abs(diffDays)} day(s) ago`;
  } catch {
    return '';
  }
}
