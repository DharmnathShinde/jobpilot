import { useLocalSearchParams, useRouter } from 'expo-router';
import { useState } from 'react';
import { KeyboardAvoidingView, Platform, Pressable, ScrollView, Text } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { useAuth } from '../src/auth';
import { Button, ErrorBanner, Field, Header } from '../src/components/ui';
import { colors, space, type } from '../src/theme';

export default function Login() {
  const params = useLocalSearchParams<{ mode?: string }>();
  const [mode, setMode] = useState<'signup' | 'signin'>(params.mode === 'signin' ? 'signin' : 'signup');
  const [name, setName] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { signIn, signUp } = useAuth();
  const router = useRouter();

  const submit = async () => {
    setBusy(true);
    setError(null);
    try {
      if (mode === 'signup') await signUp(email, password, name);
      else await signIn(email, password);
      router.replace('/');
    } catch (e) {
      setError(e instanceof Error ? (e.message === 'unauthorized' ? 'Wrong email or password' : e.message) : 'Something went wrong');
    } finally {
      setBusy(false);
    }
  };

  return (
    <SafeAreaView style={{ flex: 1 }}>
      <KeyboardAvoidingView behavior={Platform.OS === 'ios' ? 'padding' : undefined} style={{ flex: 1 }}>
        <ScrollView contentContainerStyle={{ padding: space(6) }} keyboardShouldPersistTaps="handled">
          <Header back title="" />
          <Text style={[type.display, { marginBottom: space(2) }]}>{mode === 'signup' ? 'Create your account' : 'Welcome back'}</Text>
          <Text style={[type.body, { color: colors.textSecondary, marginBottom: space(6) }]}>
            {mode === 'signup' ? 'It takes about two minutes to set up your morning queue.' : 'Sign in to see today’s matches.'}
          </Text>
          {error && <ErrorBanner message={error} />}
          {mode === 'signup' && <Field label="Name" value={name} onChangeText={setName} autoComplete="name" placeholder="Aarav Sharma" />}
          <Field label="Email" value={email} onChangeText={setEmail} autoCapitalize="none" keyboardType="email-address" autoComplete="email" placeholder="you@example.com" />
          <Field label="Password" value={password} onChangeText={setPassword} secureTextEntry autoComplete={mode === 'signup' ? 'new-password' : 'current-password'} hint={mode === 'signup' ? 'At least 8 characters' : undefined} />
          <Button label={mode === 'signup' ? 'Create account' : 'Sign in'} onPress={submit} loading={busy} disabled={!email || password.length < (mode === 'signup' ? 8 : 1)} style={{ marginTop: space(2) }} />
          <Pressable onPress={() => setMode(mode === 'signup' ? 'signin' : 'signup')} style={{ padding: space(4), alignItems: 'center' }}>
            <Text style={{ color: colors.primary, fontWeight: '600' }}>{mode === 'signup' ? 'I already have an account' : 'Create a new account'}</Text>
          </Pressable>
        </ScrollView>
      </KeyboardAvoidingView>
    </SafeAreaView>
  );
}
