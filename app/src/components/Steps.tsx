import { Text, View } from 'react-native';

import { colors, space, type } from '../theme';

export function Steps({ step }: { step: number }) {
  return (
    <View style={{ marginBottom: space(5) }}>
      <Text style={[type.caption, { fontWeight: '600', textAlign: 'right', marginBottom: 8 }]}>Step {step} of 3</Text>
      <View style={{ height: 4, borderRadius: 2, backgroundColor: colors.border }}>
        <View style={{ height: 4, borderRadius: 2, backgroundColor: colors.primary, width: `${(step / 3) * 100}%` }} />
      </View>
    </View>
  );
}
