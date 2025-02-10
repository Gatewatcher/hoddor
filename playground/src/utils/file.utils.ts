const MIME_TYPES = {
  image: {
    png: 'image/png',
    jpg: 'image/jpeg',
    jpeg: 'image/jpeg',
    gif: 'image/gif',
    webp: 'image/webp',
  },
  video: {
    mp4: 'video/mp4',
    mov: 'video/quicktime',
  },
  audio: {
    mp3: 'audio/mpeg',
    wav: 'audio/wav',
    ogg: 'audio/ogg; codecs=vorbis',
    m4a: 'audio/mp4; codecs=mp4a.40.2',
    aac: 'audio/aac',
  },
  text: {
    json: 'application/json',
    md: 'text/markdown',
    markdown: 'text/markdown',
    txt: 'text/plain',
  },
} as const;

export const arrayBufferToBase64 = (buffer: Uint8Array): string => {
  let binary = '';
  for (let i = 0; i < buffer.length; i++) {
    binary += String.fromCharCode(buffer[i]);
  }
  return btoa(binary);
};

export const getMimeTypeFromExtension = (filename: string): string => {
  const extension = filename.split('.').pop()?.toLowerCase() || '';
  
  for (const category of Object.values(MIME_TYPES)) {
    const mimeType = category[extension as keyof typeof category];
    if (mimeType) {
      return mimeType;
    }
  }

  return 'application/octet-stream';
};
