import { useRouter } from 'expo-router';
import { ScrollView, Text } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { useAuth } from '../../src/auth';
import { PreferencesEditor } from '../../src/components/PreferencesEditor';
import { Steps } from '../../src/components/Steps';
import { colors, space, type } from '../../src/theme';

export default function OnboardingPreferences() {
  const router = useRouter();
  const { refresh } = useAuth();
  return (
    <SafeAreaView style={{ flex: 1 }}>
      <ScrollView contentContainerStyle={{ padding: space(6) }} keyboardShouldPersistTaps="handled">
        <Steps step={3} />
        <Text style={type.display}>What are you looking for?</Text>
        <Text style={[type.body, { color: colors.textSecondary, marginTop: 6, marginBottom: space(5) }]}>We use this to filter and score jobs each morning.</Text>
        <PreferencesEditor
          saveLabel="Start my job search"
          onSaved={async () => {
            await refresh();
            router.replace('/today');
          }}
        />
      </ScrollView>
    </SafeAreaView>
  );
}
