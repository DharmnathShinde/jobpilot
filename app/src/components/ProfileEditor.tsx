import { Feather } from '@expo/vector-icons';
import { useEffect, useState } from 'react';
import { Pressable, Text, View } from 'react-native';

import { api, Experience, Profile } from '../api';
import { Button, Card, ErrorBanner, Field, Loading } from './ui';
import { colors, space, type } from '../theme';

/** Review the AI/rules draft. Also used later from Profile → Edit. */
export function ProfileEditor({ onSaved, saveLabel }: { onSaved: () => void; saveLabel: string }) {
  const [p, setP] = useState<Profile | null>(null);
  const [skillsText, setSkillsText] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    api.profile().then((x) => {
      setP(x);
      setSkillsText(x.skills.join(', '));
    }).catch((e) => setError(e.message));
  }, []);

  if (!p) return error ? <ErrorBanner message={error} /> : <Loading />;

  const set = <K extends keyof Profile>(k: K, v: Profile[K]) => setP({ ...p, [k]: v });
  const setExp = (i: number, patch: Partial<Experience>) => set('experience', p.experience.map((e, j) => (j === i ? { ...e, ...patch } : e)));

  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      const skills = skillsText.split(',').map((s) => s.trim()).filter(Boolean);
      const experience = p.experience
        .map((e) => ({ ...e, bullets: e.bullets.map((b) => b.trim()).filter(Boolean) }))
        .filter((e) => e.company.trim() || e.role.trim());
      await api.saveProfile({ ...p, skills, experience, confirmed: true });
      onSaved();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Save failed');
    } finally {
      setBusy(false);
    }
  };

  return (
    <View>
      <View style={{ backgroundColor: colors.primarySoft, borderRadius: 12, padding: 12, flexDirection: 'row', gap: 10, marginBottom: space(5) }}>
        <Feather name="shield" size={16} color={colors.primaryDark} />
        <Text style={{ flex: 1, color: colors.primaryDark, fontSize: 13 }}>Tailored resumes and answers only use facts from this profile.</Text>
      </View>
      {error && <ErrorBanner message={error} />}
      <Field label="Full name" value={p.full_name} onChangeText={(v) => set('full_name', v)} />
      <Field label="Headline" value={p.headline} onChangeText={(v) => set('headline', v)} placeholder="Senior Mobile Engineer" />
      <View style={{ flexDirection: 'row', gap: 12 }}>
        <View style={{ flex: 1 }}>
          <Field label="Years of experience" value={String(p.years_experience || '')} keyboardType="decimal-pad" onChangeText={(v) => set('years_experience', Number(v.replace(',', '.')) || 0)} />
        </View>
        <View style={{ flex: 1 }}>
          <Field label="Location" value={p.location} onChangeText={(v) => set('location', v)} placeholder="Pune" />
        </View>
      </View>
      <Field label="Email" value={p.email} onChangeText={(v) => set('email', v)} autoCapitalize="none" keyboardType="email-address" />
      <Field label="Phone" value={p.phone} onChangeText={(v) => set('phone', v)} keyboardType="phone-pad" />
      <Field label="Summary" value={p.summary} onChangeText={(v) => set('summary', v)} multiline placeholder="Two or three lines about your work" />
      <Field label="Skills" value={skillsText} onChangeText={setSkillsText} multiline hint="Comma-separated. Matching uses these." />

      <Text style={[type.section, { marginTop: space(2), marginBottom: space(3) }]}>Work history</Text>
      {p.experience.map((e, i) => (
        <Card key={i} style={{ marginBottom: space(3) }}>
          <View style={{ flexDirection: 'row', justifyContent: 'space-between', marginBottom: 8 }}>
            <Text style={type.overline}>ROLE {i + 1}</Text>
            <Pressable accessibilityLabel="Remove role" onPress={() => set('experience', p.experience.filter((_, j) => j !== i))}>
              <Feather name="trash-2" size={16} color={colors.textSecondary} />
            </Pressable>
          </View>
          <Field label="Company" value={e.company} onChangeText={(v) => setExp(i, { company: v })} />
          <Field label="Role" value={e.role} onChangeText={(v) => setExp(i, { role: v })} />
          <View style={{ flexDirection: 'row', gap: 12 }}>
            <View style={{ flex: 1 }}><Field label="Start" value={e.start} onChangeText={(v) => setExp(i, { start: v })} placeholder="2022" /></View>
            <View style={{ flex: 1 }}><Field label="End" value={e.end} onChangeText={(v) => setExp(i, { end: v })} placeholder="now" /></View>
          </View>
          <Field
            label="Achievements"
            value={e.bullets.join('\n')}
            onChangeText={(v) => setExp(i, { bullets: v.split('\n') })}
            multiline
            hint="One per line. These are what tailoring reorders."
          />
        </Card>
      ))}
      <Button
        label="Add a role"
        variant="soft"
        icon="plus"
        onPress={() => set('experience', [...p.experience, { company: '', role: '', start: '', end: '', bullets: [] }])}
        style={{ marginBottom: space(6) }}
      />
      <Button label={saveLabel} onPress={save} loading={busy} disabled={!p.full_name.trim()} />
    </View>
  );
}
