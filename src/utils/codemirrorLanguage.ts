import {
  LanguageDescription,
  LanguageSupport,
  StreamLanguage,
} from '@codemirror/language';
import type { Extension } from '@codemirror/state';

function legacy(parser: Parameters<typeof StreamLanguage.define>[0]) {
  return new LanguageSupport(StreamLanguage.define(parser));
}

function loadSql(dialect: 'StandardSQL') {
  return import('@codemirror/lang-sql').then((m) => m.sql({ dialect: m[dialect] }));
}

/**
 * Curated language list for SFTP editing. Avoids @codemirror/language-data, which
 * pulls in 100+ legacy modes and hundreds of Vite chunks we never use.
 */
const languages = [
  LanguageDescription.of({
    name: 'C',
    extensions: ['c', 'h', 'ino'],
    load: () => import('@codemirror/lang-cpp').then((m) => m.cpp()),
  }),
  LanguageDescription.of({
    name: 'C++',
    alias: ['cpp'],
    extensions: ['cpp', 'c++', 'cc', 'cxx', 'hpp', 'h++', 'hh', 'hxx'],
    load: () => import('@codemirror/lang-cpp').then((m) => m.cpp()),
  }),
  LanguageDescription.of({
    name: 'CSS',
    extensions: ['css'],
    load: () => import('@codemirror/lang-css').then((m) => m.css()),
  }),
  LanguageDescription.of({
    name: 'Go',
    extensions: ['go'],
    load: () => import('@codemirror/lang-go').then((m) => m.go()),
  }),
  LanguageDescription.of({
    name: 'HTML',
    alias: ['xhtml'],
    extensions: ['html', 'htm'],
    load: () => import('@codemirror/lang-html').then((m) => m.html()),
  }),
  LanguageDescription.of({
    name: 'Java',
    extensions: ['java'],
    load: () => import('@codemirror/lang-java').then((m) => m.java()),
  }),
  LanguageDescription.of({
    name: 'JavaScript',
    alias: ['ecmascript', 'js', 'node'],
    extensions: ['js', 'mjs', 'cjs'],
    load: () => import('@codemirror/lang-javascript').then((m) => m.javascript()),
  }),
  LanguageDescription.of({
    name: 'JSON',
    extensions: ['json', 'map'],
    load: () => import('@codemirror/lang-json').then((m) => m.json()),
  }),
  LanguageDescription.of({
    name: 'JSX',
    extensions: ['jsx'],
    load: () =>
      import('@codemirror/lang-javascript').then((m) => m.javascript({ jsx: true })),
  }),
  LanguageDescription.of({
    name: 'LESS',
    extensions: ['less'],
    load: () => import('@codemirror/lang-less').then((m) => m.less()),
  }),
  LanguageDescription.of({
    name: 'Markdown',
    extensions: ['md', 'markdown', 'mkd'],
    load: () => import('@codemirror/lang-markdown').then((m) => m.markdown()),
  }),
  LanguageDescription.of({
    name: 'PHP',
    extensions: ['php', 'php3', 'php4', 'php5', 'php7', 'phtml'],
    load: () => import('@codemirror/lang-php').then((m) => m.php()),
  }),
  LanguageDescription.of({
    name: 'Python',
    extensions: ['py', 'pyw'],
    load: () => import('@codemirror/lang-python').then((m) => m.python()),
  }),
  LanguageDescription.of({
    name: 'Rust',
    extensions: ['rs'],
    load: () => import('@codemirror/lang-rust').then((m) => m.rust()),
  }),
  LanguageDescription.of({
    name: 'Sass',
    extensions: ['sass'],
    load: () =>
      import('@codemirror/lang-sass').then((m) => m.sass({ indented: true })),
  }),
  LanguageDescription.of({
    name: 'SCSS',
    extensions: ['scss'],
    load: () => import('@codemirror/lang-sass').then((m) => m.sass()),
  }),
  LanguageDescription.of({
    name: 'SQL',
    extensions: ['sql'],
    load: () => loadSql('StandardSQL'),
  }),
  LanguageDescription.of({
    name: 'TSX',
    extensions: ['tsx'],
    load: () =>
      import('@codemirror/lang-javascript').then((m) =>
        m.javascript({ jsx: true, typescript: true }),
      ),
  }),
  LanguageDescription.of({
    name: 'TypeScript',
    alias: ['ts'],
    extensions: ['ts', 'mts', 'cts'],
    load: () =>
      import('@codemirror/lang-javascript').then((m) =>
        m.javascript({ typescript: true }),
      ),
  }),
  LanguageDescription.of({
    name: 'Vue',
    extensions: ['vue'],
    load: () => import('@codemirror/lang-vue').then((m) => m.vue()),
  }),
  LanguageDescription.of({
    name: 'XML',
    alias: ['rss', 'wsdl', 'xsd'],
    extensions: ['xml', 'xsl', 'xsd', 'svg'],
    load: () => import('@codemirror/lang-xml').then((m) => m.xml()),
  }),
  LanguageDescription.of({
    name: 'YAML',
    alias: ['yml'],
    extensions: ['yaml', 'yml'],
    load: () => import('@codemirror/lang-yaml').then((m) => m.yaml()),
  }),
  LanguageDescription.of({
    name: 'C#',
    alias: ['csharp', 'cs'],
    extensions: ['cs'],
    load: () =>
      import('@codemirror/legacy-modes/mode/clike').then((m) => legacy(m.csharp)),
  }),
  LanguageDescription.of({
    name: 'CMake',
    extensions: ['cmake', 'cmake.in'],
    filename: /^CMakeLists\.txt$/,
    load: () =>
      import('@codemirror/legacy-modes/mode/cmake').then((m) => legacy(m.cmake)),
  }),
  LanguageDescription.of({
    name: 'diff',
    extensions: ['diff', 'patch'],
    load: () =>
      import('@codemirror/legacy-modes/mode/diff').then((m) => legacy(m.diff)),
  }),
  LanguageDescription.of({
    name: 'Dockerfile',
    filename: /^Dockerfile$/,
    load: () =>
      import('@codemirror/legacy-modes/mode/dockerfile').then((m) =>
        legacy(m.dockerFile),
      ),
  }),
  LanguageDescription.of({
    name: 'Groovy',
    extensions: ['groovy', 'gradle'],
    filename: /^Jenkinsfile$/,
    load: () =>
      import('@codemirror/legacy-modes/mode/groovy').then((m) => legacy(m.groovy)),
  }),
  LanguageDescription.of({
    name: 'Kotlin',
    extensions: ['kt', 'kts'],
    load: () =>
      import('@codemirror/legacy-modes/mode/clike').then((m) => legacy(m.kotlin)),
  }),
  LanguageDescription.of({
    name: 'Lua',
    extensions: ['lua'],
    load: () =>
      import('@codemirror/legacy-modes/mode/lua').then((m) => legacy(m.lua)),
  }),
  LanguageDescription.of({
    name: 'Nginx',
    filename: /nginx.*\.conf$/i,
    load: () =>
      import('@codemirror/legacy-modes/mode/nginx').then((m) => legacy(m.nginx)),
  }),
  LanguageDescription.of({
    name: 'Perl',
    extensions: ['pl', 'pm'],
    load: () =>
      import('@codemirror/legacy-modes/mode/perl').then((m) => legacy(m.perl)),
  }),
  LanguageDescription.of({
    name: 'PowerShell',
    extensions: ['ps1', 'psd1', 'psm1'],
    load: () =>
      import('@codemirror/legacy-modes/mode/powershell').then((m) =>
        legacy(m.powerShell),
      ),
  }),
  LanguageDescription.of({
    name: 'Properties files',
    alias: ['ini', 'properties'],
    extensions: ['properties', 'ini', 'in'],
    load: () =>
      import('@codemirror/legacy-modes/mode/properties').then((m) =>
        legacy(m.properties),
      ),
  }),
  LanguageDescription.of({
    name: 'ProtoBuf',
    extensions: ['proto'],
    load: () =>
      import('@codemirror/legacy-modes/mode/protobuf').then((m) => legacy(m.protobuf)),
  }),
  LanguageDescription.of({
    name: 'Ruby',
    alias: ['rb', 'rake'],
    extensions: ['rb'],
    filename: /^(Gemfile|Rakefile)$/,
    load: () =>
      import('@codemirror/legacy-modes/mode/ruby').then((m) => legacy(m.ruby)),
  }),
  LanguageDescription.of({
    name: 'Shell',
    alias: ['bash', 'sh', 'zsh'],
    extensions: ['sh', 'ksh', 'bash', 'zsh'],
    filename: /^PKGBUILD$/,
    load: () =>
      import('@codemirror/legacy-modes/mode/shell').then((m) => legacy(m.shell)),
  }),
  LanguageDescription.of({
    name: 'TOML',
    extensions: ['toml'],
    load: () =>
      import('@codemirror/legacy-modes/mode/toml').then((m) => legacy(m.toml)),
  }),
];

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
