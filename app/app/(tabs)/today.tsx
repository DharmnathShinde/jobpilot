import { useRouter } from 'expo-router';
import { useState } from 'react';
import { Pressable, RefreshControl, ScrollView, Text, View } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { api } from '../../src/api';
import { useAuth } from '../../src/auth';
import { JobCard } from '../../src/components/JobCard';
import { Button, Card, Empty, ErrorBanner, Loading } from '../../src/components/ui';
import { colors, space, type } from '../../src/theme';
import { useLoad } from '../../src/useLoad';

function greeting() {
  const h = new Date().getHours();
  return h < 12 ? 'Good morning' : h < 17 ? 'Good afternoon' : 'Good evening';
}

export default function Today() {
  const router = useRouter();
  const { name } = useAuth();
  const { data, error, loading, refreshing, reload } = useLoad(() => api.today());
  const [running, setRunning] = useState(false);
  const [runMsg, setRunMsg] = useState<string | null>(null);

  const runNow = async () => {
    setRunning(true);
    setRunMsg(null);
    try {
      const r = await api.runNow();
      setRunMsg(r.errors.length ? `Added ${r.added}. ${r.errors.length} source(s) failed: ${r.errors[0]}` : `Checked ${r.fetched} jobs, added ${r.added} new matches.`);
      await reload();
    } catch (e) {
      setRunMsg(e instanceof Error ? e.message : 'Run failed');
    } finally {
      setRunning(false);
    }
  };

  const date = new Date().toLocaleDateString('en-IN', { weekday: 'long', day: 'numeric', month: 'short' });
  const first = name.split(' ')[0];
  const total = data ? data.queue_count + data.reviewed_today : 0;

  return (
    <SafeAreaView style={{ flex: 1 }} edges={['top']}>
      <ScrollView contentContainerStyle={{ padding: space(6) }} refreshControl={<RefreshControl refreshing={refreshing} onRefresh={reload} />}>
        <Text style={[type.caption, { fontWeight: '500' }]}>{date}</Text>
        <Text style={[type.display, { fontSize: 24, marginBottom: space(5) }]}>
          {greeting()}
          {first ? `, ${first}` : ''}
        </Text>

        {error && <ErrorBanner message={error} onRetry={reload} />}
        {loading && !data ? (
          <Loading />
        ) : data ? (
          <>
            <View style={{ backgroundColor: colors.primary, borderRadius: 20, padding: 20, flexDirection: 'row', alignItems: 'center', gap: 16 }}>
              <View style={{ flex: 1 }}>
                <Text style={{ color: '#C7D2FE', fontSize: 12, fontWeight: '600', letterSpacing: 0.5 }}>
                  {data.last_run ? `LAST RUN ${new Date(data.last_run.at).toLocaleTimeString('en-IN', { hour: 'numeric', minute: '2-digit' }).toUpperCase()}` : `RUNS DAILY AT ${data.run_time}`}
                </Text>
                <Text style={{ color: '#FFF', fontSize: 20, fontWeight: '700', marginTop: 6 }}>
                  {data.queue_count === 0 ? 'Your queue is clear' : `${data.queue_count} match${data.queue_count === 1 ? '' : 'es'} in your queue`}
                </Text>
                {data.queue_count > 0 && (
                  <Pressable onPress={() => router.push('/jobs')} style={{ backgroundColor: '#FFF', borderRadius: 999, paddingHorizontal: 16, paddingVertical: 9, alignSelf: 'flex-start', marginTop: 12 }}>
                    <Text style={{ color: colors.primaryDark, fontWeight: '700', fontSize: 13 }}>Review queue →</Text>
                  </Pressable>
                )}
              </View>
              <View style={{ width: 76, height: 76, borderRadius: 38, borderWidth: 6, borderColor: 'rgba(255,255,255,0.25)', alignItems: 'center', justifyContent: 'center' }}>
                <Text style={{ color: '#FFF', fontWeight: '700', fontSize: 15 }}>
                  {data.reviewed_today}/{total}
                </Text>
              </View>
            </View>

            <View style={{ flexDirection: 'row', gap: 10, marginTop: space(4) }}>
              {[
                { n: data.reviewed_today, label: 'Reviewed', c: colors.text },
                { n: data.applied_today, label: 'Applied', c: colors.success },
                { n: data.queue_count, label: 'In queue', c: colors.primary },
              ].map((s) => (
                <Card key={s.label} style={{ flex: 1, padding: 12 }}>
                  <Text style={{ fontSize: 20, fontWeight: '700', color: s.c }}>{s.n}</Text>
                  <Text style={type.caption}>{s.label}</Text>
                </Card>
              ))}
            </View>

            {runMsg && <Text style={[type.caption, { marginTop: space(3) }]}>{runMsg}</Text>}

            <View style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', marginTop: space(6), marginBottom: space(3) }}>
              <Text style={[type.title, { fontSize: 17 }]}>Top matches</Text>
              <Pressable onPress={() => router.push('/add-job')}>
                <Text style={{ color: colors.primary, fontWeight: '600' }}>+ Add a job</Text>
              </Pressable>
            </View>

            {data.top_matches.length === 0 ? (
              <Empty
                icon="sunrise"
                title="Nothing new yet"
                body="Follow a few company career pages, or add a job you saw on LinkedIn or Naukri."
                action={
                  <View style={{ gap: 10 }}>
                    <Button label="Find jobs now" onPress={runNow} loading={running} />
                    <Button label="Follow career pages" variant="soft" onPress={() => router.push('/sources')} />
                  </View>
                }
              />
            ) : (
              <>
                {data.top_matches.map((j) => (
                  <JobCard key={j.id} job={j} onPress={() => router.push(`/job/${j.id}`)} />
                ))}
                <Button label="Find more now" variant="soft" icon="refresh-cw" onPress={runNow} loading={running} style={{ marginTop: space(2) }} />
              </>
            )}
          </>
        ) : null}
      </ScrollView>
    </SafeAreaView>
  );
}
