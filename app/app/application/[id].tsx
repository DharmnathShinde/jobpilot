import * as WebBrowser from 'expo-web-browser';
import { useLocalSearchParams, useRouter } from 'expo-router';
import { useState } from 'react';
import { Pressable, ScrollView, Text, View } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { ApplicationStatus, api } from '../../src/api';
import { Button, Card, ErrorBanner, Field, Header, Loading, Logo, StatusPill } from '../../src/components/ui';
import { colors, space, statusStyle, type } from '../../src/theme';
import { useLoad } from '../../src/useLoad';

const STATUSES: ApplicationStatus[] = ['applied', 'replied', 'interview', 'offer', 'rejected'];

export default function ApplicationDetail() {
  const { id } = useLocalSearchParams<{ id: string }>();
  const router = useRouter();
  const { data, error, loading, reload } = useLoad(() => api.application(id), [id]);
  const [note, setNote] = useState('');
  const [reminder, setReminder] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const update = async (patch: Parameters<typeof api.updateApplication>[1]) => {
    setBusy(true);
    setErr(null);
    try {
      await api.updateApplication(id, patch);
      setNote('');
      await reload();
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Update failed');
    } finally {
      setBusy(false);
    }
  };

  if (loading && !data) return <SafeAreaView style={{ flex: 1 }}><Loading /></SafeAreaView>;
  if (!data) return <SafeAreaView style={{ flex: 1, padding: space(6) }}><Header back title="Application" /><ErrorBanner message={error ?? 'Not found'} /></SafeAreaView>;
  const { application: a, events } = data;
  const reminderValue = reminder ?? a.next_reminder ?? '';

  return (
    <SafeAreaView style={{ flex: 1 }}>
      <ScrollView contentContainerStyle={{ padding: space(6) }} keyboardShouldPersistTaps="handled">
        <Header back title="" />
        <View style={{ flexDirection: 'row', alignItems: 'center', gap: 14 }}>
          <Logo name={a.company || a.title} size={56} />
          <View style={{ flex: 1 }}>
            <Text style={type.title}>{a.title}</Text>
            <Text style={type.caption}>{[a.company, a.location].filter(Boolean).join(' · ')}</Text>
          </View>
          <StatusPill status={a.status} />
        </View>

        <Text style={[type.overline, { marginTop: space(6), marginBottom: 8 }]}>MOVE TO</Text>
        <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 8 }}>
          {STATUSES.map((s) => {
            const active = s === a.status;
            return (
              <Pressable
                key={s}
                disabled={active || busy}
                onPress={() => update({ status: s, event_note: note || undefined })}
                style={{ paddingHorizontal: 14, paddingVertical: 8, borderRadius: 999, borderWidth: 1, borderColor: active ? statusStyle[s].fg : colors.border, backgroundColor: active ? statusStyle[s].bg : colors.surface }}
              >
                <Text style={{ fontWeight: '600', fontSize: 13, color: active ? statusStyle[s].fg : colors.body }}>{statusStyle[s].label}</Text>
              </Pressable>
            );
          })}
        </View>

        <View style={{ height: space(5) }} />
        {err && <ErrorBanner message={err} />}
        <Field label="Follow-up reminder (YYYY-MM-DD)" value={reminderValue} onChangeText={setReminder} placeholder="2026-10-17" autoCapitalize="none" />
        <Button
          label="Save reminder"
          variant="soft"
          onPress={() => update({ next_reminder: reminderValue.trim() || null })}
          disabled={busy || !/^(\d{4}-\d{2}-\d{2})?$/.test(reminderValue.trim())}
        />

        <Text style={[type.section, { marginTop: space(6), marginBottom: space(3) }]}>Timeline</Text>
        <Card>
          {events.map((e, i) => (
            <View key={e.id} style={{ flexDirection: 'row', gap: 12, paddingBottom: i < events.length - 1 ? 14 : 0 }}>
              <View style={{ alignItems: 'center' }}>
                <View style={{ width: 10, height: 10, borderRadius: 5, backgroundColor: statusStyle[e.kind]?.fg ?? colors.textMuted, marginTop: 4 }} />
                {i < events.length - 1 && <View style={{ width: 2, flex: 1, backgroundColor: colors.border, marginTop: 4 }} />}
              </View>
              <View style={{ flex: 1 }}>
                <Text style={{ fontWeight: '600', color: colors.text }}>{statusStyle[e.kind]?.label ?? 'Note'}</Text>
                {e.note ? <Text style={type.body}>{e.note}</Text> : null}
                <Text style={type.caption}>{new Date(e.at).toLocaleString('en-IN', { day: 'numeric', month: 'short', hour: 'numeric', minute: '2-digit' })}</Text>
              </View>
            </View>
          ))}
        </Card>

        <View style={{ height: space(5) }} />
        <Field label="Add a note" value={note} onChangeText={setNote} placeholder="Recruiter called, tech round next week" />
        <Button label="Add note" variant="secondary" onPress={() => update({ event_note: note })} disabled={!note.trim() || busy} />

        <View style={{ flexDirection: 'row', gap: 10, marginTop: space(6) }}>
          <Button label="View job" variant="soft" onPress={() => router.push(`/job/${a.job_id}`)} style={{ flex: 1 }} />
          {a.url ? <Button label="Open posting ↗" variant="soft" onPress={() => WebBrowser.openBrowserAsync(a.url)} style={{ flex: 1 }} /> : null}
        </View>
      </ScrollView>
    </SafeAreaView>
  );
}
