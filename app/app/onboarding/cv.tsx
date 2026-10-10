import { Feather } from '@expo/vector-icons';
import * as DocumentPicker from 'expo-document-picker';
import { useRouter } from 'expo-router';
import { useState } from 'react';
import { Pressable, ScrollView, Text, View } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { api } from '../../src/api';
import { Steps } from '../../src/components/Steps';
import { Button, Card, Chip, ErrorBanner } from '../../src/components/ui';
import { colors, space, type } from '../../src/theme';

export default function UploadCv() {
  const router = useRouter();
  const [file, setFile] = useState<{ name: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [found, setFound] = useState<{ years: number; skills: number; method: string } | null>(null);

  const pick = async () => {
    setError(null);
    const res = await DocumentPicker.getDocumentAsync({
      type: ['application/pdf', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document', 'text/plain'],
      copyToCacheDirectory: true,
    });
    if (res.canceled) return;
    const asset = res.assets[0];
    if (asset.size && asset.size > 5 * 1024 * 1024) {
      setError('CV must be 5 MB or smaller');
      return;
    }
    setFile({ name: asset.name });
    setBusy(true);
    try {
      const out = await api.uploadCv({ uri: asset.uri, name: asset.name, mimeType: asset.mimeType, webFile: asset.file });
      setFound({ years: out.profile.years_experience, skills: out.profile.skills.length, method: out.method });
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Upload failed');
      setFile(null);
    } finally {
      setBusy(false);
    }
  };

  return (
    <SafeAreaView style={{ flex: 1 }}>
      <ScrollView contentContainerStyle={{ padding: space(6), flexGrow: 1 }}>
        <Steps step={1} />
        <Text style={type.display}>Upload your CV</Text>
        <Text style={[type.body, { color: colors.textSecondary, marginTop: 6, marginBottom: space(6) }]}>
          We’ll build your profile from it. You’ll review every detail before we use it.
        </Text>

        <Pressable
          onPress={pick}
          disabled={busy}
          accessibilityRole="button"
          style={{ height: 176, borderRadius: 20, borderWidth: 1.5, borderStyle: 'dashed', borderColor: '#A5B4FC', backgroundColor: colors.primarySoft, alignItems: 'center', justifyContent: 'center', gap: 8 }}
        >
          <View style={{ width: 52, height: 52, borderRadius: 26, backgroundColor: colors.primary, alignItems: 'center', justifyContent: 'center' }}>
            <Feather name="upload" size={22} color="#FFF" />
          </View>
          <Text style={type.section}>Tap to upload</Text>
          <Text style={type.caption}>PDF, DOCX or TXT, up to 5 MB</Text>
        </Pressable>

        {error && <View style={{ marginTop: space(4) }}><ErrorBanner message={error} /></View>}

        {file && (
          <Card style={{ marginTop: space(5) }}>
            <View style={{ flexDirection: 'row', alignItems: 'center', gap: 12 }}>
              <View style={{ width: 36, height: 36, borderRadius: 10, backgroundColor: colors.criticalSoft, alignItems: 'center', justifyContent: 'center' }}>
                <Feather name="file-text" size={16} color={colors.critical} />
              </View>
              <View style={{ flex: 1 }}>
                <Text style={{ fontWeight: '600', color: colors.text }} numberOfLines={1}>{file.name}</Text>
                <Text style={{ fontSize: 12, color: busy ? colors.primary : colors.successDark }}>{busy ? 'Reading your experience…' : 'Read successfully'}</Text>
              </View>
            </View>
            {found && (
              <View style={{ flexDirection: 'row', gap: 6, marginTop: 12, flexWrap: 'wrap' }}>
                <Chip tone="good" label={`${found.years} yrs exp`} />
                <Chip tone="good" label={`${found.skills} skills`} />
                {found.method === 'rules' && <Chip label="Add roles on the next step" />}
              </View>
            )}
          </Card>
        )}

        <View style={{ flex: 1, minHeight: space(6) }} />
        <Button label="Continue" onPress={() => router.push('/onboarding/review')} disabled={!found || busy} />
      </ScrollView>
    </SafeAreaView>
  );
}
