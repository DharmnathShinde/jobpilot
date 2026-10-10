import { useRouter } from 'expo-router';
import { ScrollView, Text } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { ProfileEditor } from '../../src/components/ProfileEditor';
import { Steps } from '../../src/components/Steps';
import { colors, space, type } from '../../src/theme';

export default function Review() {
  const router = useRouter();
  return (
    <SafeAreaView style={{ flex: 1 }}>
      <ScrollView contentContainerStyle={{ padding: space(6) }} keyboardShouldPersistTaps="handled">
        <Steps step={2} />
        <Text style={type.display}>Check your profile</Text>
        <Text style={[type.body, { color: colors.textSecondary, marginTop: 6, marginBottom: space(5) }]}>Fix anything we got wrong and add what’s missing.</Text>
        <ProfileEditor saveLabel="Looks right" onSaved={() => router.push('/onboarding/preferences')} />
      </ScrollView>
    </SafeAreaView>
  );
}
