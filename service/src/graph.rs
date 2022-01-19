//! Given a set of trace events, this will extract the graph topology
use serde::{Deserialize, Serialize};

use crate::trace::Event;

#[derive(Debug, Serialize, Deserialize)]
pub enum EdgeType {
    /// This edge describes a direct syntactic relationship between one
    /// node and another.
    Parent,

    /// This edge means that the source node used the target node for context
    /// during resolution in the compiler.  In other words, the value of the
    /// target node was used to determine the value of the source node.
    Ref,
}

/// Directed relationship of some type between two nodes in the AST.
#[derive(Debug, Serialize, Deserialize)]
pub struct Edge {
    source: usize,
    target: usize,
    ty: EdgeType,
}

/// An annotated graph of the events generated by the compiler.
#[derive(Debug, Serialize, Deserialize)]
pub struct Graph {
    edges: Vec<Edge>,
    nodes: Vec<Event>,
}

impl Graph {
    pub fn new(trace: &[Event]) -> Graph {
        info!("Construct Edge Graph");

        let mut graph = Graph {
            edges: vec![],
            nodes: trace.into(),
        };

        // Construct the set of causal edges from the event set
        graph = Self::construct_hierarchy_edges(graph);

        // Search through the set of events and there is a reference to an event
        // in the trace, then add a reference edge
        graph = Self::construct_ref_edges(graph);
        graph
    }

    /// The number of nodes in the graph
    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// The total number of edges in the graph.
    pub fn num_edges(&self) -> usize {
        self.edges.len()
    }

    /// Given the underlying trace event data, construct the set of edges which
    /// correspond to the parent->child hierarchy, where the parent event has the
    /// smallest span which contains the child event.
    fn construct_hierarchy_edges(mut graph: Graph) -> Graph {
        // Construct the set of causal edges from the event set
        let len = graph.nodes.len();
        for i in 0..len {
            // if this node has a parent id, then find its parent event and use that
            if let Some(parent_id) = graph.nodes[i].parent_id {
                for j in 0..len {
                    if graph.nodes[j].id == parent_id {
                        graph.add_parent_edge(j, i);
                        break;
                    }
                }
            } else {
                // otherwise, search via the span system
                // Find the first event that precedes event i that also contains
                // event i
                for j in i + 1..len {
                    // if the span of trace[i] is a subset of the span of trace[j]
                    if contains(&graph.nodes[j], &graph.nodes[i]) {
                        // then trace[j] is the parent of trace[i]
                        // therefore add i to the adjacency list of j
                        graph.add_parent_edge(j, i);

                        // the first trace which contains i is the parent
                        // and the search must stop otherwise ancestors will be linked
                        break;
                    }
                }
            }
        }
        graph
    }

    /// Find all the events which have contextual references to other spans and
    /// construct reference edges between the two events.  The reference edge marks
    /// a _contextual_ dependency of the source event on the target event.
    fn construct_ref_edges(mut graph: Graph) -> Graph {
        // Search through the set of events and there is a reference to an event
        // in the trace, then add a reference edge
        let len = graph.nodes.len();
        for i in 0..len {
            if let Some(ref_span) = graph.nodes[i].ref_spans {
                for j in 0..len {
                    if graph.nodes[j].source == ref_span {
                        graph.add_ref_edge(i, j);
                    }
                }
            }
        }

        graph
    }

    fn add_parent_edge(&mut self, source: usize, target: usize) {
        self.edges.push(Edge {
            source,
            target,
            ty: EdgeType::Parent,
        })
    }

    fn add_ref_edge(&mut self, source: usize, target: usize) {
        self.edges.push(Edge {
            source,
            target,
            ty: EdgeType::Ref,
        })
    }

    /// This function will merge any NOOP events, effectively removing them
    /// from the graph.
    pub fn merge_noops(&mut self) {
        println!("Merge Nodes");

        // Iterate through all nodes
        let len = self.nodes.len();
        for n in 0..len {
            // If node is a NOOP
            if self.nodes[n].error.is_none() && self.nodes[n].ok.is_none() {
                // Find the parent of this node
                let mut parent_edge_idx = 0;
                let mut parent_id = 0;
                for e in 0..self.edges.len() {
                    if self.edges[e].target == n {
                        parent_id = self.edges[e].source;
                        parent_edge_idx = e;
                        break;
                    }
                }

                // then find all edges that start at this NOOP node
                // replace their source with the parent of the NOOP node
                for e in &mut self.edges {
                    if e.source == n {
                        print!("Found NOOP ({} -> {})", e.source, e.target);
                        e.source = parent_id;
                        println!("=> Found NOOP ({} -> {})", e.source, e.target);
                    }
                }

                // delete edge connecting the parent to the NOOP node
                self.edges.remove(parent_edge_idx);
            }
        }
    }
}

/// Returns true if `a` contains `b`
fn contains(a: &Event, b: &Event) -> bool {
    let a_s = a.source;
    let b_s = b.source;
    let contains = a_s.low() <= b_s.low() && b_s.high() <= a_s.high();

    contains
}
