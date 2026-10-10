// Tokens from the JobPilot brand kit in Moda.
export const colors = {
  primary: '#4F46E5',
  primaryDark: '#3730A3',
  primarySoft: '#EEF2FF',
  text: '#0F172A',
  textSecondary: '#64748B',
  textMuted: '#94A3B8',
  body: '#334155',
  bg: '#F8FAFC',
  surface: '#FFFFFF',
  border: '#E2E8F0',
  segment: '#EEF0F4',
  chip: '#F1F5F9',
  success: '#16A34A',
  successDark: '#15803D',
  successSoft: '#DCFCE7',
  warning: '#D97706',
  warningDark: '#B45309',
  warningSoft: '#FEF3C7',
  critical: '#DC2626',
  criticalSoft: '#FEE2E2',
};

export const radius = { sm: 8, md: 12, lg: 16, xl: 20, pill: 999 };
export const space = (n: number) => n * 4;

export const type = {
  display: { fontSize: 28, fontWeight: '700' as const, color: colors.text },
  title: { fontSize: 18, fontWeight: '700' as const, color: colors.text },
  section: { fontSize: 15, fontWeight: '700' as const, color: colors.text },
  body: { fontSize: 14, color: colors.body, lineHeight: 20 },
  caption: { fontSize: 12, color: colors.textSecondary },
  overline: { fontSize: 11, fontWeight: '700' as const, color: colors.textSecondary, letterSpacing: 0.5 },
};

export function scoreColors(score: number) {
  if (score >= 80) return { fg: colors.success, bg: '#F0FDF4' };
  if (score >= 60) return { fg: colors.warning, bg: '#FFFBEB' };
  return { fg: colors.critical, bg: '#FEF2F2' };
}

export const statusStyle: Record<string, { fg: string; bg: string; label: string }> = {
  saved: { fg: colors.textSecondary, bg: colors.chip, label: 'Saved' },
  applied: { fg: '#475569', bg: colors.chip, label: 'Applied' },
  replied: { fg: colors.successDark, bg: colors.successSoft, label: 'Replied' },
  interview: { fg: colors.warningDark, bg: colors.warningSoft, label: 'Interview' },
  offer: { fg: colors.primary, bg: colors.primarySoft, label: 'Offer' },
  rejected: { fg: colors.critical, bg: colors.criticalSoft, label: 'Rejected' },
};
