import { useRouter } from 'expo-router';
import { ScrollView } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { ProfileEditor } from '../src/components/ProfileEditor';
import { Header } from '../src/components/ui';
import { space } from '../src/theme';

export default function EditProfile() {
  const router = useRouter();
  return (
    <SafeAreaView style={{ flex: 1 }}>
      <ScrollView contentContainerStyle={{ padding: space(6) }} keyboardShouldPersistTaps="handled">
        <Header back title="Master profile" subtitle="Matches are re-scored when you save" />
        <ProfileEditor saveLabel="Save profile" onSaved={() => router.back()} />
      </ScrollView>
    </SafeAreaView>
  );
}
