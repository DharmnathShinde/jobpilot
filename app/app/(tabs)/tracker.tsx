import { useRouter } from 'expo-router';
import { RefreshControl, ScrollView, Text, View } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { Application, ApplicationStatus, api } from '../../src/api';
import { timeAgo } from '../../src/components/JobCard';
import { Card, Empty, ErrorBanner, Header, Loading, Logo, StatusPill } from '../../src/components/ui';
import { colors, space, statusStyle, type } from '../../src/theme';
import { useLoad } from '../../src/useLoad';

const ORDER: ApplicationStatus[] = ['offer', 'interview', 'replied', 'applied', 'saved', 'rejected'];
const STATS: ApplicationStatus[] = ['applied', 'replied', 'interview', 'offer'];

function followUpDue(a: Application) {
  if (a.status !== 'applied' || !a.applied_at) return false;
  return Date.now() - new Date(a.applied_at).getTime() > 5 * 24 * 3600 * 1000;
}

export default function Tracker() {
  const router = useRouter();
  const { data, error, loading, refreshing, reload } = useLoad(() => api.applications());

  const groups = ORDER.map((s) => ({ status: s, items: data?.applications.filter((a) => a.status === s) ?? [] })).filter((g) => g.items.length);

  return (
    <SafeAreaView style={{ flex: 1 }} edges={['top']}>
      <ScrollView contentContainerStyle={{ padding: space(6) }} refreshControl={<RefreshControl refreshing={refreshing} onRefresh={reload} />}>
        <Header title="Tracker" />
        {error && <ErrorBanner message={error} onRetry={reload} />}
        {loading && !data ? (
          <Loading />
        ) : data ? (
          <>
            <View style={{ flexDirection: 'row', gap: 8, marginBottom: space(5) }}>
              {STATS.map((s) => (
                <Card key={s} style={{ flex: 1, padding: 12 }}>
                  <Text style={{ fontSize: 22, fontWeight: '700', color: data.counts[s] ? statusStyle[s].fg : colors.text }}>{data.counts[s]}</Text>
                  <Text style={type.caption}>{statusStyle[s].label}</Text>
                </Card>
              ))}
            </View>
            {groups.length === 0 && <Empty icon="columns" title="No applications yet" body="When you apply from a job, it shows up here so you can follow up." />}
            {groups.map((g) => (
              <View key={g.status} style={{ marginBottom: space(4) }}>
                <Text style={[type.overline, { fontSize: 13, marginBottom: space(2) }]}>
                  {statusStyle[g.status].label.toUpperCase()} · {g.items.length}
                </Text>
                {g.items.map((a) => (
                  <Card key={a.id} onPress={() => router.push(`/application/${a.id}`)} style={{ marginBottom: space(2), flexDirection: 'row', alignItems: 'center', gap: 12 }}>
                    <Logo name={a.company || a.title} />
                    <View style={{ flex: 1 }}>
                      <Text style={{ fontWeight: '700', color: colors.text }} numberOfLines={1}>{a.title}</Text>
                      <Text style={type.caption} numberOfLines={1}>
                        {[a.company, a.applied_at ? `applied ${timeAgo(a.applied_at)}` : null, a.next_reminder ? `reminder ${a.next_reminder}` : null].filter(Boolean).join(' · ')}
                      </Text>
                    </View>
                    {followUpDue(a) ? (
                      <View style={{ backgroundColor: colors.warningSoft, borderRadius: 999, paddingHorizontal: 10, paddingVertical: 4 }}>
                        <Text style={{ color: colors.warningDark, fontSize: 11, fontWeight: '700' }}>Follow up</Text>
                      </View>
                    ) : (
                      <StatusPill status={a.status} />
                    )}
                  </Card>
                ))}
              </View>
            ))}
          </>
        ) : null}
      </ScrollView>
    </SafeAreaView>
  );
}
