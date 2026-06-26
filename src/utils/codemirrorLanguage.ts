import { LanguageDescription } from '@codemirror/language';
import { languages } from '@codemirror/language-data';
import type { Extension } from '@codemirror/state';

export async function resolveLanguageForFile(fileName: string): Promise<{
  extensions: Extension[];
  languageName: string | null;
}> {
  const desc = LanguageDescription.matchFilename(languages, fileName);
  if (!desc) {
    return { extensions: [], languageName: null };
  }

  const support = await desc.load();
  return { extensions: [support], languageName: desc.name };
}
