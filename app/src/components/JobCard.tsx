import { Text, View } from 'react-native';

import { Job } from '../api';
import { colors, space } from '../theme';
import { Card, Chip, Logo, ScoreBadge, styles } from './ui';

const sourceLabel: Record<string, string> = {
  share: 'Shared by you',
  manual: 'Added by you',
  greenhouse: 'Career page',
  lever: 'Career page',
  ashby: 'Career page',
  adzuna: 'Adzuna',
  jsearch: 'Job search',
};

export function timeAgo(iso: string | null): string {
  if (!iso) return '';
  const mins = Math.max(0, Math.round((Date.now() - new Date(iso).getTime()) / 60000));
  if (mins < 1) return 'just now';
  if (mins < 60) return `${mins}m ago`;
  const h = Math.round(mins / 60);
  if (h < 24) return `${h}h ago`;
  return `${Math.round(h / 24)}d ago`;
}

export function JobCard({ job, onPress }: { job: Job; onPress: () => void }) {
  const meta = [job.company, job.location].filter(Boolean).join(' · ');
  return (
    <Card onPress={onPress} style={{ marginBottom: space(2.5) }}>
      <View style={[styles.row, { gap: 12 }]}>
        <Logo name={job.company || job.title} />
        <View style={{ flex: 1 }}>
          <Text style={{ fontSize: 14, fontWeight: '700', color: colors.text }} numberOfLines={1}>
            {job.title}
          </Text>
          {meta ? (
            <Text style={{ fontSize: 12, color: colors.textSecondary, marginTop: 2 }} numberOfLines={1}>
              {meta}
            </Text>
          ) : null}
        </View>
        <ScoreBadge score={job.score} />
      </View>
      <View style={[styles.row, { gap: 6, marginTop: 10, flexWrap: 'wrap' }]}>
        <Chip label={sourceLabel[job.source] ?? job.source} />
        {job.hr_email ? <Chip tone="good" label="HR email" /> : null}
        {(job.outreach_status ?? 'none') === 'sent' ? <Chip label="Emailed" /> : (job.outreach_status ?? 'none') === 'drafted' ? <Chip label="Draft ready" /> : null}
        {job.missing_skills.length > 0 && <Chip tone="warn" label={`${job.missing_skills.length} gap${job.missing_skills.length > 1 ? 's' : ''}`} />}
        <Chip label={timeAgo(job.posted_at ?? job.created_at)} />
      </View>
    </Card>
  );
}
