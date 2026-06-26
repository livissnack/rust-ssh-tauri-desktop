const UPPER = 'ABCDEFGHJKLMNPQRSTUVWXYZ';
const LOWER = 'abcdefghijkmnopqrstuvwxyz';
const DIGITS = '23456789';
const SYMBOLS = '!@#$%^&*-_+=?';

function secureRandomIndex(max: number): number {
  if (max <= 0) return 0;
  const maxUint = 0xffffffff;
  const limit = maxUint - (maxUint % max);
  const buf = new Uint32Array(1);
  let value = 0;
  do {
    crypto.getRandomValues(buf);
    value = buf[0];
  } while (value >= limit);
  return value % max;
}

function pickChar(pool: string): string {
  return pool[secureRandomIndex(pool.length)];
}

function shuffleChars(chars: string[]): string[] {
  for (let i = chars.length - 1; i > 0; i--) {
    const j = secureRandomIndex(i + 1);
    [chars[i], chars[j]] = [chars[j], chars[i]];
  }
  return chars;
}

/** Generate a strong random password with upper, lower, digits and symbols. */
export function generateStrongPassword(length = 30): string {
  const size = Math.max(8, length);
  const all = UPPER + LOWER + DIGITS + SYMBOLS;
  const chars: string[] = [
    pickChar(UPPER),
    pickChar(LOWER),
    pickChar(DIGITS),
    pickChar(SYMBOLS),
  ];
  for (let i = chars.length; i < size; i++) {
    chars.push(pickChar(all));
  }
  return shuffleChars(chars).join('');
}
