import { useLocalSearchParams, useRouter } from 'expo-router';
import { useState } from 'react';
import { ScrollView, Text, View } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { api } from '../../src/api';
import { timeAgo } from '../../src/components/JobCard';
import { Button, Card, Chip, ErrorBanner, Header, Loading, Logo, ScoreBadge } from '../../src/components/ui';
import { colors, space, type } from '../../src/theme';
import { useLoad } from '../../src/useLoad';

const sourceName: Record<string, string> = { share: 'shared by you', manual: 'added by you', greenhouse: 'career page', lever: 'career page', ashby: 'career page' };

function Fact({ label, value, tone }: { label: string; value: string; tone?: string }) {
  return (
    <Card style={{ flex: 1, padding: 12 }}>
      <Text style={{ fontSize: 11, color: colors.textSecondary }}>{label}</Text>
      <Text style={{ fontSize: 14, fontWeight: '700', color: tone ?? colors.text, marginTop: 3 }} numberOfLines={2}>{value}</Text>
    </Card>
  );
}

export default function JobDetail() {
  const { id } = useLocalSearchParams<{ id: string }>();
  const router = useRouter();
  const { data: job, setData, error, loading } = useLoad(() => api.job(id), [id]);
  const [busy, setBusy] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState(false);

  const setStatus = async (status: 'saved' | 'skipped' | 'new') => {
    setBusy(status);
    try {
      setData(await api.setJobStatus(id, status));
      if (status === 'skipped') router.back();
    } catch (e) {
      setActionError(e instanceof Error ? e.message : 'Failed');
    } finally {
      setBusy(null);
    }
  };

  const apply = async () => {
    setBusy('apply');
    setActionError(null);
    try {
      const r = await api.tailor(id);
      router.push(`/resume/${r.id}?job=${id}`);
    } catch (e) {
      setActionError(e instanceof Error ? e.message : 'Could not tailor resume');
    } finally {
      setBusy(null);
    }
  };

  if (loading && !job) return <SafeAreaView style={{ flex: 1 }}><Loading /></SafeAreaView>;
  if (!job) return <SafeAreaView style={{ flex: 1, padding: space(6) }}><Header back title="Job" /><ErrorBanner message={error ?? 'Not found'} /></SafeAreaView>;

  const years = job.min_years != null ? (job.max_years != null ? `${job.min_years}–${job.max_years} years` : `${job.min_years}+ years`) : 'Not stated';
  const strength = job.score >= 80 ? 'Strong match' : job.score >= 60 ? 'Good match' : 'Partial match';

  return (
    <SafeAreaView style={{ flex: 1 }}>
      <ScrollView contentContainerStyle={{ padding: space(6), paddingBottom: 140 }}>
        <Header back title="" />
        <View style={{ flexDirection: 'row', alignItems: 'center', gap: 14 }}>
          <Logo name={job.company || job.title} size={56} />
          <View style={{ flex: 1 }}>
            <Text style={type.title}>{job.title}</Text>
            <Text style={type.caption}>{[job.company, job.location].filter(Boolean).join(' · ')}</Text>
          </View>
        </View>
        <Text style={[type.caption, { marginTop: 10 }]}>via {sourceName[job.source] ?? job.source} · {timeAgo(job.posted_at ?? job.created_at)}</Text>

        <Card style={{ marginTop: space(4) }}>
          <View style={{ flexDirection: 'row', alignItems: 'center', gap: 14 }}>
            <ScoreBadge score={job.score} size={64} />
            <View style={{ flex: 1 }}>
              <Text style={type.section}>{strength}</Text>
              <Text style={[type.body, { fontSize: 13, marginTop: 2 }]}>{job.explanation}</Text>
            </View>
          </View>
          {job.matched_skills.length > 0 && (
            <>
              <Text style={[type.overline, { marginTop: 14, marginBottom: 8 }]}>YOU HAVE</Text>
              <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 6 }}>
                {job.matched_skills.map((s) => <Chip key={s} tone="good" label={s} />)}
              </View>
            </>
          )}
          {job.missing_skills.length > 0 && (
            <>
              <Text style={[type.overline, { marginTop: 12, marginBottom: 8 }]}>GAPS</Text>
              <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 6 }}>
                {job.missing_skills.map((s) => <Chip key={s} tone="warn" label={s} />)}
              </View>
            </>
          )}
        </Card>

        <View style={{ flexDirection: 'row', gap: 10, marginTop: space(4) }}>
          <Fact label="Experience asked" value={years} />
          <Fact label="Skills matched" value={job.required_skills.length ? `${job.matched_skills.length} of ${job.required_skills.length}` : '—'} tone={colors.success} />
        </View>

        <Text style={[type.section, { marginTop: space(6), marginBottom: space(2) }]}>About the role</Text>
        <Text style={[type.body, { color: '#475569' }]} numberOfLines={expanded ? undefined : 8}>{job.description}</Text>
        <Text onPress={() => setExpanded(!expanded)} style={{ color: colors.primary, fontWeight: '600', marginTop: 8 }}>{expanded ? 'Show less' : 'Read full description'}</Text>

        {actionError && <View style={{ marginTop: space(4) }}><ErrorBanner message={actionError} /></View>}
      </ScrollView>

      <View style={{ position: 'absolute', left: 0, right: 0, bottom: 0, backgroundColor: colors.surface, borderTopWidth: 1, borderTopColor: colors.border, padding: space(4), paddingBottom: space(8), flexDirection: 'row', gap: 10 }}>
        {job.queue_status === 'applied' ? (
          <Button label="Already applied · open tracker" variant="soft" onPress={() => router.push('/tracker')} style={{ flex: 1 }} />
        ) : (
          <>
            <Button label="Skip" variant="secondary" onPress={() => setStatus('skipped')} loading={busy === 'skipped'} style={{ width: 72 }} />
            <Button
              label={job.queue_status === 'saved' ? 'Saved' : 'Save'}
              variant="soft"
              onPress={() => setStatus(job.queue_status === 'saved' ? 'new' : 'saved')}
              loading={busy === 'saved' || busy === 'new'}
              style={{ width: 80 }}
            />
            <Button label="Tailor & apply" onPress={apply} loading={busy === 'apply'} style={{ flex: 1 }} />
          </>
        )}
      </View>
    </SafeAreaView>
  );
}
