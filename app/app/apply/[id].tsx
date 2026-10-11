import { Feather } from '@expo/vector-icons';
import * as Clipboard from 'expo-clipboard';
import { useLocalSearchParams, useRouter } from 'expo-router';
import * as WebBrowser from 'expo-web-browser';
import { useState } from 'react';
import { Pressable, ScrollView, Text, View } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { api } from '../../src/api';
import { Button, Card, ErrorBanner, Loading } from '../../src/components/ui';
import { colors, space, type } from '../../src/theme';
import { useLoad } from '../../src/useLoad';
import { shareResumePdf } from '../../src/pdf';

export default function Apply() {
  const { id, resume } = useLocalSearchParams<{ id: string; resume?: string }>();
  const router = useRouter();
  const { data, error, loading } = useLoad(async () => {
    const [job, answers] = await Promise.all([api.job(id), api.answers(id)]);
    return { job, answers };
  }, [id]);
  const [copied, setCopied] = useState<string | null>(null);
  const [stage, setStage] = useState<'ready' | 'confirm'>('ready');
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const copy = async (key: string, value: string) => {
    await Clipboard.setStringAsync(value);
    setCopied(key);
    setTimeout(() => setCopied((c) => (c === key ? null : c)), 1500);
  };

  const open = async () => {
    if (!data?.job.url) {
      setStage('confirm');
      return;
    }
    // Resolves when the user closes the in-app browser.
    await WebBrowser.openBrowserAsync(data.job.url);
    setStage('confirm');
  };

  const markApplied = async () => {
    setBusy(true);
    setErr(null);
    try {
      await api.createApplication(id, 'applied', resume);
      router.dismissAll();
      router.replace('/tracker');
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Could not save');
    } finally {
      setBusy(false);
    }
  };

  if (loading && !data) return <Loading />;
  if (!data) return <View style={{ padding: space(6) }}><ErrorBanner message={error ?? 'Not found'} /></View>;

  return (
    <SafeAreaView style={{ flex: 1, backgroundColor: colors.surface }} edges={['bottom']}>
      <ScrollView contentContainerStyle={{ padding: space(6) }}>
        <View style={{ width: 48, height: 5, borderRadius: 3, backgroundColor: '#CBD5E1', alignSelf: 'center', marginBottom: space(4) }} />
        {stage === 'ready' ? (
          <>
            <Text style={[type.display, { fontSize: 22 }]}>Ready to apply</Text>
            <Text style={[type.body, { color: colors.textSecondary, marginTop: 6, marginBottom: space(5) }]}>
              {data.job.url ? 'We’ll open the job page. Copy each answer when the form asks for it.' : 'This job has no link. Apply where you found it, then confirm below.'}
            </Text>

            {data.answers.map((a) => (
              <Card key={a.key} style={{ backgroundColor: colors.bg, marginBottom: 10 }}>
                <View style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', marginBottom: 6 }}>
                  <Text style={type.overline}>{a.label.toUpperCase()}</Text>
                  <Pressable onPress={() => copy(a.key, a.value)} style={{ backgroundColor: colors.primarySoft, borderRadius: 999, paddingHorizontal: 12, paddingVertical: 4 }}>
                    <Text style={{ color: colors.primary, fontWeight: '700', fontSize: 12 }}>{copied === a.key ? 'Copied ✓' : 'Copy'}</Text>
                  </Pressable>
                </View>
                <Text style={{ fontSize: 14, color: colors.text, lineHeight: 20 }} selectable>{a.value}</Text>
              </Card>
            ))}

            {resume && (
              <Pressable onPress={() => shareResumePdf(resume, data.job.company).catch((e) => setErr(e.message))}>
                <Card style={{ flexDirection: 'row', alignItems: 'center', gap: 12, backgroundColor: '#F0FDF4', borderColor: '#BBF7D0', marginBottom: space(5) }}>
                  <Feather name="download" size={20} color={colors.successDark} />
                  <View style={{ flex: 1 }}>
                    <Text style={{ fontWeight: '600', color: colors.text }}>Download tailored resume (PDF)</Text>
                    <Text style={{ fontSize: 12, color: colors.successDark }}>Tap to save or share</Text>
                  </View>
                  <Feather name="share" size={18} color={colors.successDark} />
                </Card>
              </Pressable>
            )}
            {err && <ErrorBanner message={err} />}
            <Button label={data.job.url ? 'Open job page ↗' : 'I’ve applied'} onPress={open} />
            <Button label="I’ll do it later" variant="ghost" onPress={() => router.back()} />
          </>
        ) : (
          <>
            <Text style={[type.display, { fontSize: 22 }]}>Did you submit it?</Text>
            <Text style={[type.body, { color: colors.textSecondary, marginTop: 6, marginBottom: space(6) }]}>
              We’ll add {data.job.title} at {data.job.company || 'this company'} to your tracker and remind you to follow up.
            </Text>
            {err && <ErrorBanner message={err} />}
            <Button label="Yes, I applied" onPress={markApplied} loading={busy} />
            <Button label="Not yet" variant="ghost" onPress={() => setStage('ready')} style={{ marginTop: 6 }} />
          </>
        )}
      </ScrollView>
    </SafeAreaView>
  );
}
