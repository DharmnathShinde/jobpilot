import { useRouter } from 'expo-router';
import { ScrollView } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { PreferencesEditor } from '../src/components/PreferencesEditor';
import { Header } from '../src/components/ui';
import { space } from '../src/theme';

export default function EditPreferences() {
  const router = useRouter();
  return (
    <SafeAreaView style={{ flex: 1 }}>
      <ScrollView contentContainerStyle={{ padding: space(6) }} keyboardShouldPersistTaps="handled">
        <Header back title="Preferences" />
        <PreferencesEditor saveLabel="Save" onSaved={() => router.back()} />
      </ScrollView>
    </SafeAreaView>
  );
}
