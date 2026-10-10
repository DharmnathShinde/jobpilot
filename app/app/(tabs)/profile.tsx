import { Feather } from '@expo/vector-icons';
import { useRouter } from 'expo-router';
import { Alert, Platform, Pressable, ScrollView, Text, View } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { api, API_URL } from '../../src/api';
import { useAuth } from '../../src/auth';
import { Card, Chip, ErrorBanner, Header, Loading, Logo } from '../../src/components/ui';
import { colors, space, type } from '../../src/theme';
import { useLoad } from '../../src/useLoad';

function Row({ icon, label, detail, onPress, danger }: { icon: keyof typeof Feather.glyphMap; label: string; detail?: string; onPress: () => void; danger?: boolean }) {
  return (
    <Pressable onPress={onPress} style={{ flexDirection: 'row', alignItems: 'center', gap: 12, paddingVertical: 14 }}>
      <Feather name={icon} size={18} color={danger ? colors.critical : colors.textSecondary} />
      <Text style={{ flex: 1, fontSize: 15, color: danger ? colors.critical : colors.text }}>{label}</Text>
      {detail ? <Text style={type.caption}>{detail}</Text> : null}
      {!danger && <Feather name="chevron-right" size={18} color={colors.textMuted} />}
    </Pressable>
  );
}

function confirm(title: string, message: string, ok: () => void) {
  if (Platform.OS === 'web') {
    if (globalThis.confirm?.(`${title}\n\n${message}`)) ok();
    return;
  }
  Alert.alert(title, message, [{ text: 'Cancel', style: 'cancel' }, { text: 'Delete', style: 'destructive', onPress: ok }]);
}

export default function ProfileTab() {
  const router = useRouter();
  const { signOut } = useAuth();
  const { data, error, loading, reload } = useLoad(async () => {
    const [profile, prefs, sources, catalog] = await Promise.all([api.profile(), api.preferences(), api.sources(), api.companies('', 1)]);
    return { profile, prefs, followed: catalog.total + sources.length };
  });

  return (
    <SafeAreaView style={{ flex: 1 }} edges={['top']}>
      <ScrollView contentContainerStyle={{ padding: space(6) }}>
        <Header title="Profile" />
        {error && <ErrorBanner message={error} onRetry={reload} />}
        {loading && !data ? (
          <Loading />
        ) : data ? (
          <>
            <Card style={{ flexDirection: 'row', alignItems: 'center', gap: 14 }}>
              <Logo name={data.profile.full_name || '?'} size={56} />
              <View style={{ flex: 1 }}>
                <Text style={type.title}>{data.profile.full_name}</Text>
                <Text style={type.caption}>
                  {[data.profile.headline, data.profile.years_experience ? `${data.profile.years_experience} yrs` : null].filter(Boolean).join(' · ')}
                </Text>
              </View>
            </Card>
            <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 6, marginTop: space(3) }}>
              {data.profile.skills.slice(0, 10).map((s) => (
                <Chip key={s} label={s} tone="brand" />
              ))}
              {data.profile.skills.length > 10 && <Chip label={`+${data.profile.skills.length - 10}`} />}
            </View>

            <Card style={{ marginTop: space(5), paddingVertical: 0 }}>
              <Row icon="user" label="Master profile" onPress={() => router.push('/edit-profile')} />
              <Row icon="sliders" label="Job preferences" detail={`${data.prefs.target_roles.length} roles`} onPress={() => router.push('/edit-preferences')} />
              <Row icon="clock" label="Morning run" detail={data.prefs.schedule_enabled ? data.prefs.run_time.slice(0, 5) : 'Off'} onPress={() => router.push('/edit-preferences')} />
              <Row icon="globe" label="Career pages I follow" detail={String(data.followed)} onPress={() => router.push('/sources')} />
            </Card>

            <Card style={{ marginTop: space(4), paddingVertical: 0 }}>
              <Row icon="log-out" label="Sign out" onPress={signOut} />
              <Row
                icon="trash-2"
                danger
                label="Delete account and data"
                onPress={() =>
                  confirm('Delete everything?', 'Your profile, jobs, resumes and tracker will be permanently deleted.', async () => {
                    await api.deleteAccount();
                    await signOut();
                  })
                }
              />
            </Card>
            <Text style={[type.caption, { textAlign: 'center', marginTop: space(6), color: colors.textMuted }]}>Server: {API_URL}</Text>
          </>
        ) : null}
      </ScrollView>
    </SafeAreaView>
  );
}
