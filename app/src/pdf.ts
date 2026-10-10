import * as Print from 'expo-print';
import * as Sharing from 'expo-sharing';
import { Platform } from 'react-native';

import { api } from './api';

/** Turns the server-rendered resume HTML into a PDF on the device and opens the share sheet. */
export async function shareResumePdf(resumeId: string, fileLabel: string) {
  const html = await api.resumeHtml(resumeId);
  if (Platform.OS === 'web') {
    await Print.printAsync({ html });
    return;
  }
  const { uri } = await Print.printToFileAsync({ html });
  if (await Sharing.isAvailableAsync()) {
    await Sharing.shareAsync(uri, { mimeType: 'application/pdf', dialogTitle: fileLabel, UTI: 'com.adobe.pdf' });
  }
}
