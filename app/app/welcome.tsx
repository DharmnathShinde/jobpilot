import { useRouter } from 'expo-router';
import { Pressable, Text, View } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { BrandGradient } from '../src/components/Gradient';
import { Button, Logo } from '../src/components/ui';
import { colors, space } from '../src/theme';

export default function Welcome() {
  const router = useRouter();
  return (
    <BrandGradient>
      <SafeAreaView style={{ flex: 1, paddingHorizontal: space(6) }}>
        <Text style={{ color: '#FFF', fontSize: 17, fontWeight: '700', marginTop: space(4) }}>JobPilot</Text>

        <View style={{ marginTop: space(16) }}>
          <View style={{ backgroundColor: '#FFF', borderRadius: 16, padding: 16, shadowColor: '#1E1B4B', shadowOpacity: 0.3, shadowRadius: 24, shadowOffset: { width: 0, height: 12 }, elevation: 6 }}>
            <View style={{ flexDirection: 'row', alignItems: 'center', gap: 12 }}>
              <Logo name="Kestrel Pay" />
              <View style={{ flex: 1 }}>
                <Text style={{ fontSize: 15, fontWeight: '700', color: colors.text }}>Senior React Native Engineer</Text>
                <Text style={{ fontSize: 13, color: colors.textSecondary }}>Kestrel Pay · Bengaluru · Hybrid</Text>
              </View>
              <View style={{ width: 44, height: 44, borderRadius: 22, backgroundColor: colors.successSoft, alignItems: 'center', justifyContent: 'center' }}>
                <Text style={{ color: colors.success, fontWeight: '700' }}>92</Text>
              </View>
            </View>
          </View>
          <View style={{ alignSelf: 'flex-end', backgroundColor: colors.success, borderRadius: 999, paddingHorizontal: 14, paddingVertical: 8, marginTop: space(5) }}>
            <Text style={{ color: '#FFF', fontWeight: '600', fontSize: 12 }}>Resume tailored ✓</Text>
          </View>
        </View>

        <View style={{ flex: 1 }} />
        <Text style={{ color: '#FFF', fontSize: 32, fontWeight: '700', lineHeight: 37 }}>Jobs found for you every morning</Text>
        <Text style={{ color: '#C7D2FE', fontSize: 16, lineHeight: 23, marginTop: space(4) }}>
          We check the company career pages you follow, score every role against your profile, and have it ready before your coffee.
        </Text>
        <Button label="Get started" variant="inverse" onPress={() => router.push('/login?mode=signup')} style={{ marginTop: space(8) }} />
        <Pressable onPress={() => router.push('/login?mode=signin')} style={{ padding: space(4), alignItems: 'center' }}>
          <Text style={{ color: '#E0E7FF', fontWeight: '600' }}>I have an account</Text>
        </Pressable>
      </SafeAreaView>
    </BrandGradient>
  );
}
