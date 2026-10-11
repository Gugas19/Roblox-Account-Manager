"""Lists every text the Rust app shows (interface + messages from Rust) that has no translation yet.

    python tools/strings.py            # prints the missing texts, checks every language has the same keys
    python tools/strings.py --json     # the missing texts as a JSON list
"""

import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
UI = os.path.join(ROOT, 'ui', 'src')
RUST = os.path.join(ROOT, 'src')
LANGS = os.path.join(ROOT, 'lang')
LANG = os.path.join(LANGS, 'pt-PT.json')

# props of our components whose value goes through t()
PROPS = r'(?:title|hint|label|description|confirm|cancel|empty)'
STRING = r"""(?:'((?:[^'\\]|\\.)*)'|"((?:[^"\\]|\\.)*)")"""


def unescape(s):
    return s.encode('utf-8').decode('unicode_escape').encode('latin-1').decode('utf-8') if '\\' in s else s


def ui_strings():
    found = set()
    for folder, _, files in os.walk(UI):
        if os.sep + 'ui' + os.sep + 'components' + os.sep + 'ui' in folder + os.sep:
            continue  # shadcn's own components
        for name in files:
            if not name.endswith(('.ts', '.tsx')):
                continue
            text = open(os.path.join(folder, name), encoding='utf-8').read()
            patterns = [
                r'\bt\(\s*' + STRING,                          # t('text')
                r'\b(?:notify\.\w+|toast)\(\s*' + STRING,       # notify.success('text')
                r'\b' + PROPS + r'=' + STRING,                  # title="text"
                r'\b' + PROPS + r':\s*' + STRING,               # { title: 'text' }
                r"\[\s*'\w+'\s*,\s*" + STRING + r'\s*\]',       # ['AccentColor', 'Accent']
                r'\bname:\s*' + STRING,                         # presets
            ]
            for p in patterns:
                for m in re.finditer(p, text):
                    s = m.group(1) if m.group(1) is not None else m.group(2)
                    if is_text(s):
                        found.add(unescape(s))
    return found


def is_text(s):
    """Interface text, not an identifier, class name, path or url"""
    if not s or not re.search('[A-Za-z]{2}', s) or s.startswith(('http', '#', '@/', './', 'w-', 'h-', 'size-')):
        return False
    if ' ' in s.strip():
        return not re.search(r'(?:flex|grid|text-|bg-|px-|py-|gap-|rounded|border)', s)  # tailwind classes
    return bool(re.fullmatch(r'[A-Z][a-z]+(?:-[A-Za-z]+)?', s))  # one word: Add, See-through (not AccentColor)


def rust_strings():
    """Messages Rust sends to the interface: errors, toasts, results (format! placeholders become {0}, {1}...)"""
    found = set()
    for folder, _, files in os.walk(RUST):
        if os.path.relpath(folder, RUST).split(os.sep)[0] == 'cli':
            continue  # the command line prints English for scripts, nothing there reaches the interface
        for name in files:
            if not name.endswith('.rs'):
                continue
            text = open(os.path.join(folder, name), encoding='utf-8').read()
            text = re.sub(r'#\[cfg\(test\)\].*', '', text, flags=re.S)
            for m in re.finditer(r'(?:bail!|anyhow!|toast\([^,]+,|Err\(|format!|JoinResult::Error\()\s*\(?\s*"((?:[^"\\]|\\.)*)"', text):
                s = m.group(1)
                if not re.search('[A-Za-z]{3}', s) or s.startswith(('http', '{', 'ws:', 'roblox-player', '<roblox')) or name in ('logging.rs',):
                    continue
                n = iter(range(100))
                s = re.sub(r'\{[^{}]*\}', lambda _: '{%d}' % next(n), s)
                if is_code(s):
                    continue
                found.add(s.replace('\\n', '\n').replace('\\"', '"'))
            # results of commands, shown as toasts (the web server answers other programs, it stays in English)
            if os.sep + 'commands' in folder or 'antiafk' in folder:
                for m in re.finditer(r'"([A-Z][^"\\]*)"\.into\(\)', text):
                    if ' ' in m.group(1) or is_text(m.group(1)):
                        found.add(m.group(1))
    return found


def is_code(s):
    """Command line flags, javascript, urls and file names built with format!, never shown as text"""
    return (s.startswith(('-', '&', '.', '!', 'document.', 'ram-', 'chrome-', 'Win64-', '0:'))
            or ' ' not in s.strip()
            or '\\r' in s)


IGNORE = {
    'Roblox Account Manager', 'Discord', 'URL', 'PID', 'Main', 'Builder', 'Default', 'Ctrl K', 'Anti-AFK', 'proxies.txt', 'Roblox',
    'ABC123', 'Missing Content', 'Missing RAMHeader', 'encryption failed', 'unknown', 'no stable version', 'no win64 download', 'browser error',
    'script error', 'no version', 'Method `{0}` not allowed',
}

def check_languages():
    """Every language has the same keys as pt-PT and keeps the {0} placeholders"""
    problems = []
    reference = json.load(open(LANG, encoding='utf-8'))
    for name in sorted(os.listdir(LANGS)):
        table = json.load(open(os.path.join(LANGS, name), encoding='utf-8'))
        if set(table) != set(reference):
            problems.append(f'{name}: keys differ from pt-PT.json ({len(set(table) ^ set(reference))})')
        for key, value in table.items():
            if sorted(re.findall(r'\{\d+\}', key)) != sorted(re.findall(r'\{\d+\}', value)):
                problems.append(f'{name}: placeholders differ in {key!r}')
    return problems


if __name__ == '__main__':
    table = json.load(open(LANG, encoding='utf-8'))
    every = sorted((ui_strings() | rust_strings()) - IGNORE)
    missing = [s for s in every if s not in table]
    if '--json' in sys.argv:
        print(json.dumps(missing, ensure_ascii=False, indent=1))
    else:
        print(f'{len(every)} texts, {len(missing)} without translation')
        for s in missing:
            print('  ' + repr(s))
        problems = check_languages()
        for p in problems:
            print('  ' + p)
        sys.exit(1 if missing or problems else 0)
