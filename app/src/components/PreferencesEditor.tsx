import { useEffect, useState } from 'react';
import { Pressable, Switch, Text, View } from 'react-native';

import { api, Preferences } from '../api';
import { Button, ErrorBanner, Field, Loading } from './ui';
import { colors, space, type } from '../theme';

const DAYS = ['M', 'T', 'W', 'T', 'F', 'S', 'S'];
const list = (s: string) => s.split(',').map((x) => x.trim()).filter(Boolean);

export function PreferencesEditor({ onSaved, saveLabel }: { onSaved: () => void; saveLabel: string }) {
  const [p, setP] = useState<Preferences | null>(null);
  const [roles, setRoles] = useState('');
  const [locs, setLocs] = useState('');
  const [time, setTime] = useState('07:00');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    api.preferences().then((x) => {
      setP(x);
      setRoles(x.target_roles.join(', '));
      setLocs(x.locations.join(', '));
      setTime(x.run_time.slice(0, 5));
    }).catch((e) => setError(e.message));
  }, []);

  if (!p) return error ? <ErrorBanner message={error} /> : <Loading />;
  const set = <K extends keyof Preferences>(k: K, v: Preferences[K]) => setP({ ...p, [k]: v });

  const save = async () => {
    if (!/^([01]\d|2[0-3]):[0-5]\d$/.test(time)) {
      setError('Run time must look like 07:00');
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await api.savePreferences({ ...p, target_roles: list(roles), locations: list(locs), run_time: `${time}:00` });
      onSaved();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Save failed');
    } finally {
      setBusy(false);
    }
  };

  return (
    <View>
      {error && <ErrorBanner message={error} />}
      <Field label="Target roles" value={roles} onChangeText={setRoles} placeholder="React Native Engineer, Mobile Engineer" hint="Comma-separated. Only jobs with these words in the title are collected." />
      <Field label="Locations" value={locs} onChangeText={setLocs} placeholder="Dubai, UAE, Bengaluru" hint="Dubai also matches UAE / Abu Dhabi / Sharjah postings." />
      <View style={{ flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between', marginBottom: space(4) }}>
        <Text style={type.body}>Include remote roles</Text>
        <Switch value={p.remote_ok} onValueChange={(v) => set('remote_ok', v)} trackColor={{ true: colors.primary }} />
      </View>
      <View style={{ flexDirection: 'row', gap: 12 }}>
        <View style={{ flex: 1 }}>
          <Field label="Expected CTC (₹ LPA)" value={p.expected_ctc_lpa?.toString() ?? ''} keyboardType="decimal-pad" onChangeText={(v) => set('expected_ctc_lpa', v ? Number(v) || null : null)} />
        </View>
        <View style={{ flex: 1 }}>
          <Field label="Notice (days)" value={p.notice_days?.toString() ?? ''} keyboardType="number-pad" onChangeText={(v) => set('notice_days', v ? parseInt(v, 10) || null : null)} />
        </View>
      </View>
      <View style={{ flexDirection: 'row', gap: 12 }}>
        <View style={{ flex: 1 }}>
          <Field label="Minimum match score" value={String(p.min_score)} keyboardType="number-pad" onChangeText={(v) => set('min_score', Math.min(100, parseInt(v, 10) || 0))} hint="Jobs below this score stay out of your queue." />
        </View>
        <View style={{ flex: 1 }}>
          <Field
            label="Max jobs per run"
            value={String(p.max_jobs_per_run)}
            keyboardType="number-pad"
            onChangeText={(v) => set('max_jobs_per_run', Math.min(1000, Math.max(1, parseInt(v, 10) || 1)))}
            hint="How many new matches to keep each morning (up to 1000)."
          />
        </View>
      </View>

      <Text style={[type.section, { marginTop: space(2), marginBottom: space(3) }]}>Morning run</Text>
      <View style={{ flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between', marginBottom: space(4) }}>
        <Text style={type.body}>Find jobs automatically</Text>
        <Switch value={p.schedule_enabled} onValueChange={(v) => set('schedule_enabled', v)} trackColor={{ true: colors.primary }} />
      </View>
      <Field label="Run time (24h)" value={time} onChangeText={setTime} placeholder="07:00" hint={`Timezone: ${p.timezone}`} />
      <Text style={[type.overline, { marginBottom: 8 }]}>ACTIVE DAYS</Text>
      <View style={{ flexDirection: 'row', gap: 8, marginBottom: space(6) }}>
        {DAYS.map((d, i) => {
          const day = i + 1;
          const on = p.active_days.includes(day);
          return (
            <Pressable
              key={day}
              accessibilityRole="checkbox"
              accessibilityState={{ checked: on }}
              onPress={() => set('active_days', on ? p.active_days.filter((x) => x !== day) : [...p.active_days, day].sort())}
              style={{ flex: 1, height: 40, borderRadius: 10, alignItems: 'center', justifyContent: 'center', backgroundColor: on ? colors.primary : colors.surface, borderWidth: 1, borderColor: on ? colors.primary : colors.border }}
            >
              <Text style={{ fontWeight: '700', color: on ? '#FFF' : colors.textSecondary }}>{d}</Text>
            </Pressable>
          );
        })}
      </View>
      <Button label={saveLabel} onPress={save} loading={busy} />
    </View>
  );
}
