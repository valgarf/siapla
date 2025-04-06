import { date } from 'quasar';

export function format_datetime(dt: Date | null | undefined): string {
  if (dt == null) {
    return '-';
  }
  return date.formatDate(dt, 'YYYY-MM-DD HH:mm');
}
