import { Feather } from '@expo/vector-icons';
import { Redirect, Tabs } from 'expo-router';
import { ColorValue, Platform } from 'react-native';

import { useAuth } from '../../src/auth';
import { colors } from '../../src/theme';

export default function TabsLayout() {
  const { ready, signedIn } = useAuth();
  if (ready && !signedIn) return <Redirect href="/welcome" />;
  const icon = (name: keyof typeof Feather.glyphMap) => ({ color }: { color: ColorValue }) => <Feather name={name} size={22} color={color} />;
  return (
    <Tabs
      screenOptions={{
        headerShown: false,
        tabBarActiveTintColor: colors.primary,
        tabBarInactiveTintColor: colors.textMuted,
        tabBarStyle: { borderTopColor: colors.border, ...(Platform.OS === 'web' ? { height: 64, paddingBottom: 8, paddingTop: 6 } : null) },
        tabBarLabelStyle: { fontSize: 11, fontWeight: '600' },
        sceneStyle: { backgroundColor: colors.bg },
      }}
    >
      <Tabs.Screen name="today" options={{ title: 'Today', tabBarIcon: icon('home') }} />
      <Tabs.Screen name="jobs" options={{ title: 'Jobs', tabBarIcon: icon('briefcase') }} />
      <Tabs.Screen name="tracker" options={{ title: 'Tracker', tabBarIcon: icon('columns') }} />
      <Tabs.Screen name="profile" options={{ title: 'Profile', tabBarIcon: icon('user') }} />
    </Tabs>
  );
}
