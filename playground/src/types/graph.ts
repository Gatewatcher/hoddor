export interface GraphNodeResult {
  id: string;
  node_type: string;
  content: string;
  labels: string[];
  similarity: number | null;
}

export interface GraphNodeWithNeighborsResult {
  id: string;
  node_type: string;
  content: string;
  labels: string[];
  similarity: number;
  neighbors: GraphNodeResult[];
}
