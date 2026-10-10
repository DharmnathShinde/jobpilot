import { Feather } from '@expo/vector-icons';
import { useLocalSearchParams, useRouter } from 'expo-router';
import { useState } from 'react';
import { Pressable, ScrollView, Text, View } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { api, Source } from '../src/api';
import { Button, Card, ErrorBanner, Field, Header, Loading, Logo, Segmented } from '../src/components/ui';
import { colors, space, type } from '../src/theme';
import { useLoad } from '../src/useLoad';

const KINDS: { value: Source['kind']; label: string; example: string }[] = [
  { value: 'greenhouse', label: 'Greenhouse', example: 'boards.greenhouse.io/<slug>' },
  { value: 'lever', label: 'Lever', example: 'jobs.lever.co/<slug>' },
  { value: 'ashby', label: 'Ashby', example: 'jobs.ashbyhq.com/<slug>' },
];

/** Pulls the board slug and platform out of a pasted careers URL. */
function parseCareersUrl(text: string): { kind: Source['kind']; board: string } | null {
  const m = text.match(/(greenhouse\.io|lever\.co|ashbyhq\.com)\/(?:embed\/job_board\?for=)?([A-Za-z0-9_.-]+)/);
  if (!m) return null;
  const kind = m[1].startsWith('greenhouse') ? 'greenhouse' : m[1].startsWith('lever') ? 'lever' : 'ashby';
  return { kind, board: m[2] };
}

export default function Sources() {
  const { first } = useLocalSearchParams<{ first?: string }>();
  const router = useRouter();
  const { data, setData, error, loading } = useLoad(() => api.sources());
  const [kind, setKind] = useState<Source['kind']>('greenhouse');
  const [board, setBoard] = useState('');
  const [company, setCompany] = useState('');
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const onBoardChange = (v: string) => {
    const parsed = parseCareersUrl(v);
    if (parsed) {
      setKind(parsed.kind);
      setBoard(parsed.board);
    } else setBoard(v);
  };

  const add = async () => {
    setBusy(true);
    setErr(null);
    try {
      const s = await api.addSource(kind, board.trim(), company.trim() || board.trim());
      setData([...(data ?? []).filter((x) => x.id !== s.id), s]);
      setBoard('');
      setCompany('');
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Could not add');
    } finally {
      setBusy(false);
    }
  };

  const remove = async (id: string) => {
    await api.deleteSource(id);
    setData((data ?? []).filter((s) => s.id !== id));
  };

  return (
    <SafeAreaView style={{ flex: 1 }}>
      <ScrollView contentContainerStyle={{ padding: space(6) }} keyboardShouldPersistTaps="handled">
        <Header back={!first} title="Career pages" subtitle="Checked every morning" />
        <Text style={[type.body, { color: colors.textSecondary, marginBottom: space(5) }]}>
          Many companies list jobs on Greenhouse, Lever or Ashby. Paste a company’s careers link and we’ll check it every morning. For LinkedIn and Naukri, use “Add a job”.
        </Text>

        {error && <ErrorBanner message={error} />}
        {loading && !data ? <Loading /> : null}
        {(data ?? []).map((s) => (
          <Card key={s.id} style={{ flexDirection: 'row', alignItems: 'center', gap: 12, marginBottom: 8 }}>
            <Logo name={s.company || s.board} size={36} />
            <View style={{ flex: 1 }}>
              <Text style={{ fontWeight: '700', color: colors.text }}>{s.company || s.board}</Text>
              <Text style={type.caption}>{KINDS.find((k) => k.value === s.kind)?.label} · {s.board}</Text>
            </View>
            <Pressable accessibilityLabel={`Stop following ${s.company}`} onPress={() => remove(s.id)} hitSlop={8}>
              <Feather name="x" size={18} color={colors.textSecondary} />
            </Pressable>
          </Card>
        ))}

        <Text style={[type.section, { marginTop: space(4), marginBottom: space(3) }]}>Follow a company</Text>
        <Segmented value={kind} onChange={setKind} options={KINDS.map(({ value, label }) => ({ value, label }))} />
        <View style={{ height: space(4) }} />
        {err && <ErrorBanner message={err} />}
        <Field label="Careers link or slug" value={board} onChangeText={onBoardChange} autoCapitalize="none" placeholder={KINDS.find((k) => k.value === kind)?.example} />
        <Field label="Company name" value={company} onChangeText={setCompany} placeholder="Shown on job cards" />
        <Button label="Follow" variant="soft" icon="plus" onPress={add} loading={busy} disabled={!board.trim()} />

        {first && (
          <Button label={data?.length ? 'Done' : 'Skip for now'} onPress={() => router.replace('/today')} style={{ marginTop: space(8) }} />
        )}
      </ScrollView>
    </SafeAreaView>
  );
}
