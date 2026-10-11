import * as Print from 'expo-print';
import * as Sharing from 'expo-sharing';
import { Platform } from 'react-native';

import { api } from './api';

function safeFileName(label: string): string {
  const base = label.replace(/[^\w\s\-.–—]/g, '').trim() || 'resume';
  return `${base.slice(0, 80)}.pdf`;
}

/** Turns the server-rendered resume HTML into a PDF and lets the user save/share it. */
export async function shareResumePdf(resumeId: string, fileLabel: string) {
  const html = await api.resumeHtml(resumeId);
  const { uri } = await Print.printToFileAsync({ html });

  if (Platform.OS === 'web') {
    try {
      const res = await fetch(uri);
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = safeFileName(fileLabel);
      a.rel = 'noopener';
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
      return;
    } catch {
      await Print.printAsync({ html });
      return;
    }
  }

  if (!(await Sharing.isAvailableAsync())) {
    throw new Error('Sharing is not available on this device. Open the resume on another device to save the PDF.');
  }
  await Sharing.shareAsync(uri, {
    mimeType: 'application/pdf',
    dialogTitle: fileLabel,
    UTI: 'com.adobe.pdf',
  });
}
