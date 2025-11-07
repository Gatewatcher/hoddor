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
