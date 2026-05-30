import re

with open(r'F:\workspace\colorTicket\src\App.vue', 'r', encoding='utf-8') as f:
    content = f.read()

# Find safeInvoke and replace
old = '''async function safeInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch {
    return mockInvoke<T>(command, args);
  }
}'''

new = '''async function safeInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const inTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;
  if (!inTauri) {
    return mockInvoke<T>(command, args);
  }
  try {
    return await Promise.race([
      invoke<T>(command, args),
      new Promise<never>((_, reject) => setTimeout(() => reject(new Error('timeout')), 2000)),
    ]);
  } catch {
    return mockInvoke<T>(command, args);
  }
}'''

if old in content:
    content = content.replace(old, new)
    print('Replaced safeInvoke')
else:
    print('Could not find safeInvoke to replace')
    # Print surrounding context for debugging
    idx = content.find('async function safeInvoke')
    if idx >= 0:
        print('Found at index', idx)
        print(repr(content[idx:idx+200]))

with open(r'F:\workspace\colorTicket\src\App.vue', 'w', encoding='utf-8') as f:
    f.write(content)
