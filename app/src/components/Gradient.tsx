import { LinearGradient } from 'expo-linear-gradient';
import { ReactNode } from 'react';

import { colors } from '../theme';

export function BrandGradient({ children }: { children: ReactNode }) {
  return (
    <LinearGradient colors={[colors.primary, colors.primaryDark]} style={{ flex: 1 }}>
      {children}
    </LinearGradient>
  );
}
