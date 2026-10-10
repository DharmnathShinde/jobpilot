import { useLocalSearchParams, useRouter } from 'expo-router';
import { useState } from 'react';
import { KeyboardAvoidingView, Platform, ScrollView, Text } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { api } from '../src/api';
import { Button, ErrorBanner, Field } from '../src/components/ui';
import { colors, space, type } from '../src/theme';

/**
 * Add a job found on LinkedIn, Naukri or Indeed. Also opened by deep link:
 * jobpilot://add-job?url=...&title=...&company=...
 */
export default function AddJob() {
  const params = useLocalSearchParams<{ url?: string; title?: string; company?: string }>();
  const router = useRouter();
  const [title, setTitle] = useState(params.title ?? '');
  const [company, setCompany] = useState(params.company ?? '');
  const [location, setLocation] = useState('');
  const [url, setUrl] = useState(params.url ?? '');
  const [description, setDescription] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async () => {
    setBusy(true);
    setError(null);
    try {
      const job = await api.addJob({ title, company, location, url, description, via: params.url ? 'share' : 'manual' });
      router.replace(`/job/${job.id}`);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Could not add job');
    } finally {
      setBusy(false);
    }
  };

  return (
    <SafeAreaView style={{ flex: 1, backgroundColor: colors.surface }} edges={['bottom']}>
      <KeyboardAvoidingView behavior={Platform.OS === 'ios' ? 'padding' : undefined} style={{ flex: 1 }}>
        <ScrollView contentContainerStyle={{ padding: space(6) }} keyboardShouldPersistTaps="handled">
          <Text style={[type.display, { fontSize: 22 }]}>Add a job</Text>
          <Text style={[type.body, { color: colors.textSecondary, marginTop: 6, marginBottom: space(5) }]}>
            Found something on LinkedIn, Naukri or Indeed? Paste it here and we’ll score it and tailor your resume.
          </Text>
          {error && <ErrorBanner message={error} />}
          <Field label="Job title" value={title} onChangeText={setTitle} placeholder="Senior React Native Engineer" />
          <Field label="Company" value={company} onChangeText={setCompany} />
          <Field label="Location" value={location} onChangeText={setLocation} placeholder="Bengaluru · Hybrid" />
          <Field label="Link" value={url} onChangeText={setUrl} autoCapitalize="none" keyboardType="url" placeholder="https://" />
          <Field label="Job description" value={description} onChangeText={setDescription} multiline placeholder="Paste the full description" style={{ minHeight: 160 }} />
          <Button label="Analyse match" onPress={submit} loading={busy} disabled={!title.trim() || description.trim().length < 30} />
        </ScrollView>
      </KeyboardAvoidingView>
    </SafeAreaView>
  );
}
