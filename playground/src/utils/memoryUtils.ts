export const encodeContent = (content: string): Uint8Array => {
  const encoder = new TextEncoder();
  return encoder.encode(content);
};

export const decodeContent = (contentBytes: Uint8Array): string => {
  const decoder = new TextDecoder();
  return decoder.decode(contentBytes);
};

export const generateHMAC = async (content: Uint8Array): Promise<string> => {
  const hmac = await crypto.subtle.digest('SHA-256', content);
  return Array.from(new Uint8Array(hmac))
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
};

export const parseLabels = (labelsString: string): string[] => {
  return labelsString
    .split(',')
    .map(l => l.trim())
    .filter(l => l.length > 0);
};

export const createMemory = (
  id: string,
  content: string,
  labels: string[],
): {
  id: string;
  content: string;
  labels: string[];
  timestamp: Date;
} => {
  return {
    id,
    content,
    labels,
    timestamp: new Date(),
  };
};
