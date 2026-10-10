import { Feather } from '@expo/vector-icons';
import { useLocalSearchParams, useRouter } from 'expo-router';
import { useState } from 'react';
import { ScrollView, Text, View } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { api, Profile } from '../../src/api';
import { Button, Card, ErrorBanner, Header, IconButton, Loading } from '../../src/components/ui';
import { colors, space, type } from '../../src/theme';
import { shareResumePdf } from '../../src/pdf';
import { useLoad } from '../../src/useLoad';

function Preview({ p, highlight }: { p: Profile; highlight: string[] }) {
  const hl = highlight.map((h) => h.toLowerCase());
  return (
    <Card style={{ padding: 20, borderRadius: 12 }}>
      <Text style={{ fontSize: 16, fontWeight: '700', color: colors.text }}>{p.full_name}</Text>
      <Text style={{ fontSize: 11, color: colors.textSecondary, marginTop: 2 }}>{[p.headline, p.location].filter(Boolean).join(' · ')}</Text>
      <View style={{ height: 1, backgroundColor: colors.border, marginVertical: 10 }} />
      {p.summary ? (
        <>
          <Text style={{ fontSize: 10, fontWeight: '700', color: colors.primary, letterSpacing: 0.6 }}>SUMMARY</Text>
          <Text style={{ fontSize: 11, color: colors.body, lineHeight: 16, marginTop: 4, marginBottom: 10 }}>{p.summary}</Text>
        </>
      ) : null}
      <Text style={{ fontSize: 10, fontWeight: '700', color: colors.primary, letterSpacing: 0.6 }}>SKILLS</Text>
      <Text style={{ fontSize: 11, lineHeight: 16, marginTop: 4, marginBottom: 10 }}>
        {p.skills.map((s, i) => (
          <Text key={s} style={{ color: hl.includes(s.toLowerCase()) ? colors.primaryDark : colors.body, fontWeight: hl.includes(s.toLowerCase()) ? '700' : '400' }}>
            {s}
            {i < p.skills.length - 1 ? ', ' : ''}
          </Text>
        ))}
      </Text>
      {p.experience.slice(0, 2).map((e) => (
        <View key={e.company + e.role} style={{ marginBottom: 8 }}>
          <Text style={{ fontSize: 11, fontWeight: '700', color: colors.text }}>
            {e.role}, {e.company} · {[e.start, e.end].filter(Boolean).join('–')}
          </Text>
          {e.bullets.slice(0, 3).map((b) => (
            <Text key={b} style={{ fontSize: 11, color: colors.body, lineHeight: 16 }}>• {b}</Text>
          ))}
        </View>
      ))}
      {p.experience.length > 2 && <Text style={{ fontSize: 11, color: colors.textMuted }}>…</Text>}
    </Card>
  );
}

export default function ResumeScreen() {
  const { id, job } = useLocalSearchParams<{ id: string; job?: string }>();
  const router = useRouter();
  const { data, error, loading } = useLoad(async () => {
    const r = await api.resume(id);
    const j = await api.job(job ?? r.job_id);
    return { r, j };
  }, [id]);
  const [sharing, setSharing] = useState(false);
  const [shareError, setShareError] = useState<string | null>(null);

  if (loading && !data) return <SafeAreaView style={{ flex: 1 }}><Loading /></SafeAreaView>;
  if (!data) return <SafeAreaView style={{ flex: 1, padding: space(6) }}><Header back title="Resume" /><ErrorBanner message={error ?? 'Not found'} /></SafeAreaView>;
  const { r, j } = data;

  const share = async () => {
    setSharing(true);
    setShareError(null);
    try {
      await shareResumePdf(r.id, `${r.content.full_name} – ${j.company}`);
    } catch (e) {
      setShareError(e instanceof Error ? e.message : 'Could not create PDF');
    } finally {
      setSharing(false);
    }
  };

  return (
    <SafeAreaView style={{ flex: 1 }}>
      <ScrollView contentContainerStyle={{ padding: space(6), paddingBottom: 130 }}>
        <Header back title="Tailored resume" subtitle={`For ${j.company || 'this role'} · ${j.title}`} right={<IconButton icon="download" label="Download PDF" onPress={share} />} />
        <Preview p={r.content} highlight={j.matched_skills} />

        <Card style={{ marginTop: space(4) }}>
          <View style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center' }}>
            <Text style={type.section}>ATS check</Text>
            <View style={{ backgroundColor: r.keyword_coverage >= 70 ? colors.successSoft : colors.warningSoft, borderRadius: 999, paddingHorizontal: 10, paddingVertical: 4 }}>
              <Text style={{ fontSize: 11, fontWeight: '700', color: r.keyword_coverage >= 70 ? colors.successDark : colors.warningDark }}>
                {r.keyword_coverage >= 70 ? 'Passing' : 'Some gaps'}
              </Text>
            </View>
          </View>
          <View style={{ flexDirection: 'row', justifyContent: 'space-between', marginTop: 12 }}>
            <Text style={type.body}>Keyword coverage</Text>
            <Text style={{ fontWeight: '700', color: colors.text }}>{r.keyword_coverage}%</Text>
          </View>
          <View style={{ height: 6, borderRadius: 3, backgroundColor: colors.border, marginTop: 8 }}>
            <View style={{ height: 6, borderRadius: 3, width: `${r.keyword_coverage}%`, backgroundColor: r.keyword_coverage >= 70 ? colors.success : colors.warning }} />
          </View>
          <Text style={{ color: colors.successDark, fontSize: 12, marginTop: 10 }}>✓ Single-column layout parses cleanly</Text>
          <Text style={{ color: colors.successDark, fontSize: 12, marginTop: 2 }}>✓ Uses only facts from your profile</Text>
        </Card>

        <Text style={[type.section, { marginTop: space(6), marginBottom: space(3) }]}>What changed</Text>
        {r.changes.map((c, i) => (
          <Card key={i} style={{ flexDirection: 'row', alignItems: 'center', gap: 10, marginBottom: 8, padding: 12 }}>
            <View style={{ width: 22, height: 22, borderRadius: 6, backgroundColor: c.kind === 'not_added' ? colors.warningSoft : colors.primarySoft, alignItems: 'center', justifyContent: 'center' }}>
              {c.kind === 'not_added' ? <Feather name="alert-triangle" size={12} color={colors.warningDark} /> : <Text style={{ fontSize: 11, fontWeight: '700', color: colors.primary }}>{i + 1}</Text>}
            </View>
            <Text style={[type.body, { flex: 1, fontSize: 13 }]}>{c.text}</Text>
          </Card>
        ))}
        {shareError && <ErrorBanner message={shareError} />}
      </ScrollView>

      <View style={{ position: 'absolute', left: 0, right: 0, bottom: 0, backgroundColor: colors.surface, borderTopWidth: 1, borderTopColor: colors.border, padding: space(4), paddingBottom: space(8), flexDirection: 'row', gap: 10 }}>
        <Button label="Edit profile" variant="secondary" onPress={() => router.push('/edit-profile')} style={{ width: 120 }} />
        <Button label="Use this resume" onPress={() => router.push(`/apply/${j.id}?resume=${r.id}`)} loading={sharing} style={{ flex: 1 }} />
      </View>
    </SafeAreaView>
  );
}
