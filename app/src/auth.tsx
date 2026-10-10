import * as SecureStore from 'expo-secure-store';
import { createContext, ReactNode, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { Platform } from 'react-native';

import { api, setToken, setUnauthorizedHandler } from './api';

const KEY = 'jobpilot.token';

// SecureStore is native-only; the web build falls back to localStorage.
const store = {
  get: async () => (Platform.OS === 'web' ? globalThis.localStorage?.getItem(KEY) ?? null : SecureStore.getItemAsync(KEY)),
  set: async (v: string) => (Platform.OS === 'web' ? globalThis.localStorage?.setItem(KEY, v) : SecureStore.setItemAsync(KEY, v)),
  clear: async () => (Platform.OS === 'web' ? globalThis.localStorage?.removeItem(KEY) : SecureStore.deleteItemAsync(KEY)),
};

type AuthState = {
  ready: boolean;
  signedIn: boolean;
  profileConfirmed: boolean;
  name: string;
  signIn: (email: string, password: string) => Promise<void>;
  signUp: (email: string, password: string, name: string) => Promise<void>;
  signOut: () => Promise<void>;
  refresh: () => Promise<void>;
};

const Ctx = createContext<AuthState | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [ready, setReady] = useState(false);
  const [signedIn, setSignedIn] = useState(false);
  const [profileConfirmed, setProfileConfirmed] = useState(false);
  const [name, setName] = useState('');

  const signOut = useCallback(async () => {
    await store.clear();
    setToken(null);
    setSignedIn(false);
    setProfileConfirmed(false);
  }, []);

  const refresh = useCallback(async () => {
    const me = await api.me();
    setName(me.name);
    setProfileConfirmed(me.profile_confirmed);
  }, []);

  useEffect(() => {
    setUnauthorizedHandler(() => void signOut());
    (async () => {
      const t = await store.get();
      if (t) {
        setToken(t);
        try {
          await refresh();
          setSignedIn(true);
        } catch {
          await signOut();
        }
      }
      setReady(true);
    })();
  }, [refresh, signOut]);

  const finish = useCallback(
    async (t: string) => {
      await store.set(t);
      setToken(t);
      await refresh();
      setSignedIn(true);
    },
    [refresh],
  );

  const value = useMemo<AuthState>(
    () => ({
      ready,
      signedIn,
      profileConfirmed,
      name,
      signIn: async (email, password) => finish((await api.login(email, password)).token),
      signUp: async (email, password, n) => finish((await api.register(email, password, n)).token),
      signOut,
      refresh,
    }),
    [ready, signedIn, profileConfirmed, name, finish, signOut, refresh],
  );
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useAuth() {
  const v = useContext(Ctx);
  if (!v) throw new Error('useAuth must be used inside AuthProvider');
  return v;
}
