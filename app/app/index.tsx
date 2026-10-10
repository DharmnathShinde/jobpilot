import { Redirect } from 'expo-router';
import { ActivityIndicator, View } from 'react-native';

import { useAuth } from '../src/auth';
import { colors } from '../src/theme';

export default function Index() {
  const { ready, signedIn, profileConfirmed } = useAuth();
  if (!ready) {
    return (
      <View style={{ flex: 1, alignItems: 'center', justifyContent: 'center' }}>
        <ActivityIndicator color={colors.primary} />
      </View>
    );
  }
  if (!signedIn) return <Redirect href="/welcome" />;
  if (!profileConfirmed) return <Redirect href="/onboarding/cv" />;
  return <Redirect href="/today" />;
}
