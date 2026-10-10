import { Feather } from '@expo/vector-icons';
import { useRouter } from 'expo-router';
import { ReactNode } from 'react';
import {
  ActivityIndicator,
  Pressable,
  StyleProp,
  StyleSheet,
  Text,
  TextInput,
  TextInputProps,
  View,
  ViewStyle,
} from 'react-native';

import { colors, radius, scoreColors, space, statusStyle, type } from '../theme';

type ButtonProps = {
  label: string;
  onPress: () => void;
  variant?: 'primary' | 'secondary' | 'ghost' | 'soft' | 'inverse';
  loading?: boolean;
  disabled?: boolean;
  style?: StyleProp<ViewStyle>;
  icon?: keyof typeof Feather.glyphMap;
};

export function Button({ label, onPress, variant = 'primary', loading, disabled, style, icon }: ButtonProps) {
  const v = buttonVariants[variant];
  const off = disabled || loading;
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityState={{ disabled: !!off }}
      onPress={onPress}
      disabled={off}
      style={({ pressed }) => [styles.button, { backgroundColor: v.bg, opacity: off ? 0.5 : pressed ? 0.85 : 1 }, style]}
    >
      {loading ? (
        <ActivityIndicator color={v.fg} />
      ) : (
        <View style={styles.row}>
          {icon && <Feather name={icon} size={16} color={v.fg} style={{ marginRight: 6 }} />}
          <Text style={[styles.buttonText, { color: v.fg }]}>{label}</Text>
        </View>
      )}
    </Pressable>
  );
}

const buttonVariants = {
  primary: { bg: colors.primary, fg: '#FFFFFF' },
  secondary: { bg: colors.chip, fg: '#475569' },
  soft: { bg: colors.primarySoft, fg: colors.primary },
  ghost: { bg: 'transparent', fg: colors.textSecondary },
  inverse: { bg: '#FFFFFF', fg: colors.primaryDark },
};

export function Card({ children, style, onPress }: { children: ReactNode; style?: StyleProp<ViewStyle>; onPress?: () => void }) {
  if (onPress) {
    return (
      <Pressable onPress={onPress} style={({ pressed }) => [styles.card, pressed && { opacity: 0.9 }, style]}>
        {children}
      </Pressable>
    );
  }
  return <View style={[styles.card, style]}>{children}</View>;
}

export function Chip({ label, tone = 'neutral' }: { label: string; tone?: 'neutral' | 'good' | 'warn' | 'brand' }) {
  const t = {
    neutral: { bg: colors.chip, fg: '#475569' },
    good: { bg: colors.successSoft, fg: colors.successDark },
    warn: { bg: colors.warningSoft, fg: colors.warningDark },
    brand: { bg: colors.primarySoft, fg: colors.primary },
  }[tone];
  return (
    <View style={[styles.chip, { backgroundColor: t.bg }]}>
      <Text style={[styles.chipText, { color: t.fg }]}>{label}</Text>
    </View>
  );
}

export function ScoreBadge({ score, size = 40 }: { score: number; size?: number }) {
  const c = scoreColors(score);
  return (
    <View
      accessibilityLabel={`Match score ${score} out of 100`}
      style={{ width: size, height: size, borderRadius: size / 2, borderWidth: size > 50 ? 5 : 3, borderColor: c.fg, backgroundColor: c.bg, alignItems: 'center', justifyContent: 'center' }}
    >
      <Text style={{ color: c.fg, fontWeight: '700', fontSize: size > 50 ? 20 : 13 }}>{score}</Text>
    </View>
  );
}

export function StatusPill({ status }: { status: string }) {
  const s = statusStyle[status] ?? statusStyle.saved;
  return (
    <View style={[styles.chip, { backgroundColor: s.bg }]}>
      <Text style={[styles.chipText, { color: s.fg, fontWeight: '700' }]}>{s.label}</Text>
    </View>
  );
}

export function Logo({ name, size = 40 }: { name: string; size?: number }) {
  const initials = (name || '?')
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => w[0]?.toUpperCase())
    .join('');
  const palette = [
    ['#EEF2FF', colors.primary],
    ['#ECFEFF', '#0E7490'],
    ['#FEF2F2', '#B91C1C'],
    ['#EDE9FE', '#6D28D9'],
    ['#F0FDF4', colors.successDark],
    ['#FFF7ED', '#C2410C'],
  ];
  const [bg, fg] = palette[[...name].reduce((a, c) => a + c.charCodeAt(0), 0) % palette.length];
  return (
    <View style={{ width: size, height: size, borderRadius: size * 0.25, backgroundColor: bg, alignItems: 'center', justifyContent: 'center' }}>
      <Text style={{ color: fg, fontWeight: '700', fontSize: size * 0.33 }}>{initials}</Text>
    </View>
  );
}

export function Segmented<T extends string>({ value, options, onChange }: { value: T; options: { value: T; label: string }[]; onChange: (v: T) => void }) {
  return (
    <View style={styles.segment}>
      {options.map((o) => {
        const active = o.value === value;
        return (
          <Pressable key={o.value} onPress={() => onChange(o.value)} style={[styles.segmentItem, active && styles.segmentActive]} accessibilityRole="tab" accessibilityState={{ selected: active }}>
            <Text style={{ fontSize: 13, fontWeight: active ? '700' : '500', color: active ? colors.text : colors.textSecondary }}>{o.label}</Text>
          </Pressable>
        );
      })}
    </View>
  );
}

export function Field({ label, hint, ...props }: TextInputProps & { label: string; hint?: string }) {
  return (
    <View style={{ marginBottom: space(4) }}>
      <Text style={[type.overline, { marginBottom: 6 }]}>{label.toUpperCase()}</Text>
      <TextInput placeholderTextColor={colors.textMuted} {...props} style={[styles.input, props.multiline && { minHeight: 100, textAlignVertical: 'top' }, props.style]} />
      {hint && <Text style={[type.caption, { marginTop: 4 }]}>{hint}</Text>}
    </View>
  );
}

export function Header({ title, subtitle, right, back }: { title: string; subtitle?: string; right?: ReactNode; back?: boolean }) {
  const router = useRouter();
  return (
    <View style={[styles.row, { paddingVertical: space(3), gap: space(3) }]}>
      {back && (
        <Pressable accessibilityLabel="Back" onPress={() => router.back()} style={styles.iconButton}>
          <Feather name="arrow-left" size={18} color={colors.text} />
        </Pressable>
      )}
      <View style={{ flex: 1 }}>
        <Text style={back ? type.title : type.display} numberOfLines={1}>
          {title}
        </Text>
        {subtitle ? <Text style={type.caption} numberOfLines={1}>{subtitle}</Text> : null}
      </View>
      {right}
    </View>
  );
}

export function IconButton({ icon, onPress, label }: { icon: keyof typeof Feather.glyphMap; onPress: () => void; label: string }) {
  return (
    <Pressable accessibilityLabel={label} onPress={onPress} style={styles.iconButton}>
      <Feather name={icon} size={17} color={colors.text} />
    </Pressable>
  );
}

export function Empty({ icon, title, body, action }: { icon: keyof typeof Feather.glyphMap; title: string; body: string; action?: ReactNode }) {
  return (
    <View style={{ alignItems: 'center', paddingVertical: space(10), paddingHorizontal: space(6) }}>
      <View style={{ width: 56, height: 56, borderRadius: 28, backgroundColor: colors.primarySoft, alignItems: 'center', justifyContent: 'center', marginBottom: space(3) }}>
        <Feather name={icon} size={24} color={colors.primary} />
      </View>
      <Text style={[type.section, { textAlign: 'center' }]}>{title}</Text>
      <Text style={[type.body, { textAlign: 'center', color: colors.textSecondary, marginTop: 4 }]}>{body}</Text>
      {action && <View style={{ marginTop: space(4), alignSelf: 'stretch' }}>{action}</View>}
    </View>
  );
}

export function ErrorBanner({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <View style={[styles.card, { backgroundColor: '#FEF2F2', borderColor: '#FECACA', flexDirection: 'row', alignItems: 'center', gap: 10 }]}>
      <Feather name="alert-circle" size={18} color={colors.critical} />
      <Text style={[type.body, { flex: 1, color: '#991B1B' }]}>{message}</Text>
      {onRetry && (
        <Pressable onPress={onRetry}>
          <Text style={{ color: colors.critical, fontWeight: '700' }}>Retry</Text>
        </Pressable>
      )}
    </View>
  );
}

export function Loading() {
  return (
    <View style={{ padding: space(10), alignItems: 'center' }}>
      <ActivityIndicator color={colors.primary} />
    </View>
  );
}

export const styles = StyleSheet.create({
  row: { flexDirection: 'row', alignItems: 'center' },
  button: { height: 52, borderRadius: 14, alignItems: 'center', justifyContent: 'center', paddingHorizontal: space(4) },
  buttonText: { fontSize: 15, fontWeight: '700' },
  card: { backgroundColor: colors.surface, borderColor: colors.border, borderWidth: 1, borderRadius: radius.lg, padding: 14 },
  chip: { borderRadius: radius.pill, paddingHorizontal: 10, paddingVertical: 4, alignSelf: 'flex-start' },
  chipText: { fontSize: 11, fontWeight: '600' },
  segment: { flexDirection: 'row', backgroundColor: colors.segment, borderRadius: radius.md, padding: 4, gap: 4 },
  segmentItem: { flex: 1, height: 34, borderRadius: 9, alignItems: 'center', justifyContent: 'center' },
  segmentActive: { backgroundColor: colors.surface, shadowColor: '#0F172A', shadowOpacity: 0.08, shadowRadius: 3, shadowOffset: { width: 0, height: 1 }, elevation: 1 },
  input: { backgroundColor: colors.surface, borderColor: colors.border, borderWidth: 1, borderRadius: radius.md, paddingHorizontal: 14, paddingVertical: 12, fontSize: 15, color: colors.text },
  iconButton: { width: 40, height: 40, borderRadius: 12, borderWidth: 1, borderColor: colors.border, backgroundColor: colors.surface, alignItems: 'center', justifyContent: 'center' },
});
