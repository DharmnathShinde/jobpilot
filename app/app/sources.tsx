import { Feather } from '@expo/vector-icons';
import { useEffect, useState } from 'react';
import { Pressable, ScrollView, Text, View } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { api, Company, CompanyRequest, Source } from '../src/api';
import { Button, Card, Chip, ErrorBanner, Field, Header, Loading, Logo, Segmented } from '../src/components/ui';
import { colors, space, type } from '../src/theme';
import { useLoad } from '../src/useLoad';

const KINDS: { value: Source['kind']; label: string; example: string }[] = [
  { value: 'greenhouse', label: 'Greenhouse', example: 'boards.greenhouse.io/<slug>' },
  { value: 'lever', label: 'Lever', example: 'jobs.lever.co/<slug>' },
  { value: 'ashby', label: 'Ashby', example: 'jobs.ashbyhq.com/<slug>' },
];
const kindLabel = (k: string) => KINDS.find((x) => x.value === k)?.label ?? k;

const PAGE = 50;

const REQUEST_STATUS: Record<CompanyRequest['status'], { label: string; tone: 'warn' | 'good' | 'neutral' }> = {
  pending: { label: 'Looking…', tone: 'warn' },
  added: { label: 'Added', tone: 'good' },
  not_found: { label: 'Not found', tone: 'neutral' },
};

/** Pulls the board slug and platform out of a pasted careers URL. */
function parseCareersUrl(text: string): { kind: Source['kind']; board: string } | null {
  const m = text.match(/(greenhouse\.io|lever\.co|ashbyhq\.com)\/(?:embed\/job_board\?for=)?([A-Za-z0-9_.-]+)/);
  if (!m) return null;
  const kind = m[1].startsWith('greenhouse') ? 'greenhouse' : m[1].startsWith('lever') ? 'lever' : 'ashby';
  return { kind, board: m[2] };
}

export default function Sources() {
  const { data, setData, error, loading } = useLoad(async () => {
    const [sources, requests] = await Promise.all([api.sources(), api.companyRequests()]);
    return { sources, requests };
  });

  // Shared catalog, searchable and paged.
  const [search, setSearch] = useState('');
  const [catalog, setCatalog] = useState<{ total: number; companies: Company[] } | null>(null);
  const [catalogErr, setCatalogErr] = useState<string | null>(null);
  const [moreBusy, setMoreBusy] = useState(false);
  useEffect(() => {
    let live = true;
    const t = setTimeout(() => {
      api
        .companies(search.trim(), PAGE)
        .then((c) => live && (setCatalog(c), setCatalogErr(null)))
        .catch((e) => live && setCatalogErr(e instanceof Error ? e.message : 'Could not load companies'));
    }, search ? 300 : 0);
    return () => {
      live = false;
      clearTimeout(t);
    };
  }, [search]);
  const showMore = async () => {
    if (!catalog) return;
    setMoreBusy(true);
    try {
      const next = await api.companies(search.trim(), PAGE, catalog.companies.length);
      setCatalog({ total: next.total, companies: [...catalog.companies, ...next.companies] });
    } finally {
      setMoreBusy(false);
    }
  };

  // "Add more companies": a request the cron resolves.
  const [requesting, setRequesting] = useState(false);
  const [query, setQuery] = useState('');
  const [reqBusy, setReqBusy] = useState(false);
  const [reqMsg, setReqMsg] = useState<string | null>(null);
  const [reqErr, setReqErr] = useState<string | null>(null);
  const request = async () => {
    setReqBusy(true);
    setReqErr(null);
    setReqMsg(null);
    try {
      const r = await api.requestCompany(query.trim());
      if (data) setData({ ...data, requests: [r, ...data.requests] });
      setReqMsg(
        r.status === 'added'
          ? `${r.company_name ?? 'That company'} is already on your list.`
          : 'Got it. We’ll find its career page and add it, usually within an hour.',
      );
      setQuery('');
    } catch (e) {
      setReqErr(e instanceof Error ? e.message : 'Could not send');
    } finally {
      setReqBusy(false);
    }
  };

  // Manual: a page followed only by this user.
  const [manual, setManual] = useState(false);
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
      if (data) setData({ ...data, sources: [...data.sources.filter((x) => x.id !== s.id), s] });
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
    if (data) setData({ ...data, sources: data.sources.filter((s) => s.id !== id) });
  };

  return (
    <SafeAreaView style={{ flex: 1 }}>
      <ScrollView contentContainerStyle={{ padding: space(6) }} keyboardShouldPersistTaps="handled">
        <Header back title="Career pages" subtitle="Checked every morning" />

        <Card style={{ backgroundColor: colors.primarySoft, borderColor: colors.primarySoft }}>
          <Text style={[type.display, { color: colors.primary }]}>{catalog && !search ? catalog.total : '500+'}</Text>
          <Text style={{ fontWeight: '700', color: colors.text, marginTop: 2 }}>companies followed for you</Text>
          <Text style={[type.caption, { marginTop: 6 }]}>
            Every morning we check their career pages and score new roles against your profile. Nothing to set up.
          </Text>
        </Card>

        <Button
          label="Add more companies"
          icon="plus"
          variant={requesting ? 'soft' : 'primary'}
          onPress={() => setRequesting(!requesting)}
          style={{ marginTop: space(4) }}
        />
        {requesting && (
          <View style={{ marginTop: space(4) }}>
            {reqErr && <ErrorBanner message={reqErr} />}
            <Field
              label="Company name or careers link"
              value={query}
              onChangeText={setQuery}
              autoCapitalize="words"
              placeholder="e.g. Razorpay or razorpay.com/careers"
              hint="We find its career page and add it to the list for everyone."
            />
            <Button label="Add company" variant="soft" onPress={request} loading={reqBusy} disabled={query.trim().length < 2} />
          </View>
        )}
        {reqMsg && <Text style={[type.caption, { marginTop: space(3) }]}>{reqMsg}</Text>}

        {error && <ErrorBanner message={error} />}
        {loading && !data ? <Loading /> : null}

        {!!data?.requests.length && (
          <>
            <Text style={[type.section, { marginTop: space(6), marginBottom: space(3) }]}>Your requests</Text>
            {data.requests.map((r) => (
              <Card key={r.id} style={{ flexDirection: 'row', alignItems: 'center', gap: 12, marginBottom: 8 }}>
                <View style={{ flex: 1 }}>
                  <Text style={{ fontWeight: '600', color: colors.text }} numberOfLines={1}>{r.company_name ?? r.query}</Text>
                  {r.status === 'not_found' && <Text style={type.caption}>Try pasting its careers link instead</Text>}
                </View>
                <Chip label={REQUEST_STATUS[r.status].label} tone={REQUEST_STATUS[r.status].tone} />
              </Card>
            ))}
          </>
        )}

        <Text style={[type.section, { marginTop: space(6), marginBottom: space(3) }]}>Companies we check</Text>
        <Field label="Search" value={search} onChangeText={setSearch} autoCapitalize="none" placeholder="Company name" />
        {catalogErr && <ErrorBanner message={catalogErr} />}
        {!catalog && !catalogErr ? <Loading /> : null}
        {catalog?.companies.map((c) => (
          <View key={c.id} style={{ flexDirection: 'row', alignItems: 'center', gap: 12, paddingVertical: 8 }}>
            <Logo name={c.name} size={32} />
            <View style={{ flex: 1 }}>
              <Text style={{ fontWeight: '600', color: colors.text }}>{c.name}</Text>
              <Text style={type.caption}>
                {kindLabel(c.kind)} · {c.last_checked_at ? `${c.job_count} open roles` : 'checking soon'}
              </Text>
            </View>
          </View>
        ))}
        {catalog && catalog.companies.length === 0 && (
          <Text style={type.caption}>Not on the list yet. Use “Add more companies” above.</Text>
        )}
        {catalog && catalog.companies.length < catalog.total && (
          <Button label={`Show more (${catalog.total - catalog.companies.length} left)`} variant="ghost" onPress={showMore} loading={moreBusy} />
        )}

        <Text style={[type.section, { marginTop: space(6), marginBottom: space(3) }]}>Only for you</Text>
        {data?.sources.map((s) => (
          <Card key={s.id} style={{ flexDirection: 'row', alignItems: 'center', gap: 12, marginBottom: 8 }}>
            <Logo name={s.company || s.board} size={36} />
            <View style={{ flex: 1 }}>
              <Text style={{ fontWeight: '700', color: colors.text }}>{s.company || s.board}</Text>
              <Text style={type.caption}>{kindLabel(s.kind)} · {s.board}</Text>
            </View>
            <Pressable accessibilityLabel={`Stop following ${s.company}`} onPress={() => remove(s.id)} hitSlop={8}>
              <Feather name="x" size={18} color={colors.textSecondary} />
            </Pressable>
          </Card>
        ))}
        {manual ? (
          <>
            <Segmented value={kind} onChange={setKind} options={KINDS.map(({ value, label }) => ({ value, label }))} />
            <View style={{ height: space(4) }} />
            {err && <ErrorBanner message={err} />}
            <Field label="Careers link or slug" value={board} onChangeText={onBoardChange} autoCapitalize="none" placeholder={KINDS.find((k) => k.value === kind)?.example} />
            <Field label="Company name" value={company} onChangeText={setCompany} placeholder="Shown on job cards" />
            <Button label="Follow" variant="soft" icon="plus" onPress={add} loading={busy} disabled={!board.trim()} />
          </>
        ) : (
          <Button label="Add a career page manually" variant="ghost" icon="link" onPress={() => setManual(true)} />
        )}
      </ScrollView>
    </SafeAreaView>
  );
}
