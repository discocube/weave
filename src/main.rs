use ndarray::{Array2, Axis, Slice};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

pub mod graphs;
pub mod operators;
pub mod structs;
pub mod utils;

use crate::graphs::data::g_280::{VERTS, ADJ, EDGES};
use crate::graphs::make::make::{make_weights, make_vi_mapping, make_edges_adj, graph_to_map, shrink_adjacency, translate_verts_3d, convert_from_nodes};
use crate::graphs::info::certify::id_seq;
use crate::graphs::info::certify::{SequenceID, SequenceID::HamCycle};
use crate::operators::operators::{cut, spin, wind};
use crate::operators::operators::color;
use crate::structs::vector::Vector3D;
use crate::structs::cycle::Cycle;
use crate::utils::time::elapsed_ms;

type Adjacency = HashMap<u32, HashSet<u32>>;
type Bobbins = Vec<u32>;
type Loom = Vec<VecDeque<u32>>;
type WarpedLoom<'a> = HashMap<usize, &'a mut Cycle<'a>>;
type Spool = HashMap<u32, Array2<i32>>;
type Vert2d = (i32, i32);
type Edge = (u32, u32);
type Edges = HashSet<Edge>;
type EdgeAdjacency = HashMap<Edge, HashSet<Edge>>;
type Path = Vec<u32>;
type Processed = HashSet<usize>;
type Solution = Vec<u32>;
type Thread = VecDeque<u32>;
type Vectors3d = Vec<Vector3D>;
type VertIdx<'a> = HashMap<&'a Vector3D, u32>;
type Verts2d = Vec<Vert2d>;
type Wefts = Vec<VecDeque<u32>>;
type Weights = HashMap<u32, i32>;
type Yarn = Array2<i32>;

const REPEATS: u32 = 10_000;

fn main() {
    let adj: Adjacency = graph_to_map(&ADJ);
    let v3verts: Vectors3d = translate_verts_3d(&VERTS);
    let vert_idx: VertIdx = make_vi_mapping(&v3verts);
    let edge_adj: EdgeAdjacency = make_edges_adj(&adj, &EDGES.iter().cloned().collect::<Edges>());
    let mut solution: Solution = Vec::new();

    let start: Instant = Instant::now();
    for _i in 0..=REPEATS { 
        solution = weave(&v3verts, &adj, &vert_idx, &edge_adj) 
    }
    elapsed_ms(start, Instant:: now(), REPEATS, "weave");

    let id: SequenceID = id_seq(&solution, &adj);
    assert_eq!(HamCycle, id);
    println!("{:?}", id);
    println!("⭕️ ORDER: {:?} | ID: {:?} | {:?}", ADJ.len(), id, solution);
}

fn weave(v3verts: &Vectors3d, adj: &Adjacency, vert_idx: &VertIdx, edge_adj: &EdgeAdjacency) -> Solution {
    let mut warp_wefts: Wefts = warp_loom(v3verts, &adj, vert_idx);
    let (warp, wefts) = warp_wefts.split_first_mut().unwrap();
    let warp: &mut Cycle = Cycle::new(warp, &adj, &edge_adj);
    let loom: WarpedLoom = wefts.iter().enumerate().map(|(idx, seq)| (idx, Cycle::new(&seq, &adj, &edge_adj))).collect();
    let mut processed: Processed = HashSet::new();
    if loom.keys().len() > 0 {
        'weaving: loop {
            for idx in loom.keys() {
                if processed.len() == loom.keys().len() { break 'weaving };
                if processed.len() - 1 == loom.keys().len() { warp.set_last() };
                if processed.contains(idx) { continue };
                let mut bridge: Edges = warp.edges().intersection(&loom[idx].eadjs()).into_iter().cloned().collect::<Edges>();
                if !bridge.is_empty() {
                    let warp_e: Edge = bridge.drain().next().unwrap();
                    let mut other: Cycle = loom[&*idx].clone();
                    let mut weft_es: Edges = edge_adj.get(&warp_e).unwrap().intersection(&other.edges()).into_iter().cloned().collect::<Edges>();
                    if !weft_es.is_empty() {
                        let weft_e: Edge = weft_es.drain().next().unwrap();
                        warp.join(warp_e, weft_e, &mut other);
                        processed.extend([idx]);
                    }
                }
            }
        }
    }
    warp.retrieve()
}
    
fn warp_loom(v3verts: &Vectors3d, adj: &Adjacency, vert_idx: &VertIdx) -> Loom {
    let (z_adj, z_length) = shrink_adjacency(&v3verts, &adj);
    let spool: Spool = spool_yarn(&z_adj);
    let mut bobbins: Bobbins = Vec::new();
    let mut warps: Vec<Vec<u32>>;
    let mut loom: Loom = Vec::new();
    for (zlevel, order) in z_length {
        let mut yarn: Yarn = spool[&(zlevel % 4 + 4).try_into().unwrap()].clone();
        yarn.slice_axis_inplace(Axis(0), Slice::new((yarn.len_of(Axis(0)) - order).try_into().unwrap(), None, 1));
        let node_yarn: Vec<u32> = yarn.outer_iter().map(|row| Vector3D::to_node(row[0], row[1], zlevel, &vert_idx)).collect();
        if bobbins.is_empty() { warps = vec![node_yarn] } else { warps = cut(node_yarn, &bobbins) }
        let mut woven: Processed = HashSet::new();
        for thread in &mut loom {
            for (idx, warp) in warps.iter().enumerate() {
                if !woven.contains(&idx) {
                    for end in vec![0 as usize, thread.len() - 1] {
                        if thread[end] == warp[0 as usize] {
                            woven.extend([idx]);
                            for node in &warp[1..] {
                                if end == 0 as usize { thread.push_front(*node) } else { thread.push_back(*node) }
                            }
                        }
                    }
                }
            }
        }
        for (_, seq) in warps.iter().enumerate().filter(|(idx, _)| !woven.contains(idx)) {
            loom.extend(vec![VecDeque::from(seq.iter().cloned().collect::<Thread>())]);
        }
        let v3verts: &Vectors3d = &translate_verts_3d(&VERTS);
        if zlevel != -1 { bobbins = wind(&mut loom, v3verts, &vert_idx) }
    }
    for w in &mut loom {
        let nodes: Path = w.iter().map(|&node| v3verts[node as usize].mirror_z(&vert_idx)).collect();
        w.extend(nodes.into_iter().rev());
    }
    loom.sort_by_key(|w| w.len());
    loom
}

fn spool_yarn(z_adj: &Adjacency) -> Spool {
    let verts: &Verts2d = &VERTS.iter().clone().map(|&(x, y, _)| (x, y)).collect::<Verts2d>();
    let weights: Weights = make_weights(&z_adj, &VERTS);
    let natural: Yarn = convert_from_nodes(spin(&z_adj, &weights), &verts);
    let colored: Yarn = color(&natural);
    HashMap::from([(3, natural), (1, colored)])
}