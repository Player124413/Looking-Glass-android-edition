use super::*;

// Preserve the old broad phase and traversal order as an independent oracle.
fn old_visit(node: &Node, query: Bounds, f: &mut impl FnMut(usize)) {
    match node {
        Node::Leaf { bounds, hulls } => {
            if bounds.overlaps(query) { for &i in hulls { f(i); } }
        }
        Node::Branch { bounds, left, right } => {
            if bounds.overlaps(query) { old_visit(left, query, f); old_visit(right, query, f); }
        }
    }
}
fn original(w: &World, a: Vec3, b: Vec3, half: Vec3) -> Trace {
    let q=Bounds { min:a.min(b)-half-Vec3::splat(SKIN), max:a.max(b)+half+Vec3::splat(SKIN) };
    let mut t=Trace::default();
    if let Some(tree)=&w.tree {
        old_visit(tree,q,&mut |i| { let h=&w.hulls[i]; if h.bounds.overlaps(q) {clip_hull(h,a,b,half,&mut t);} });
    }
    t
}
fn compare(w: &World, count: usize) {
    let mut seed=0x8e93c021_u32;
    let mut random=|| { seed=seed.wrapping_mul(1664525).wrapping_add(1013904223); (seed >> 8) as f32 / 16777216. };
    for i in 0..count {
        let mut point=|| w.min+(w.max-w.min)*Vec3::new(random(),random(),random());
        let a=point();
        let mut b=point();
        if i%4==0 {b=a;} else if i%4==1 {b.x=a.x;b.y=a.y;}
        let half=[Vec3::ZERO,PLAYER_HALF,Vec3::new(44.,44.,23.)][i%3];
        let expected=original(w,a,b,half);
        let got=w.ledge_trace(a,b,half);
        assert_eq!((got.fraction,got.normal,got.start_solid,got.all_solid),
                   (expected.fraction,expected.normal,expected.start_solid,expected.all_solid),"trace {i}: {a:?} -> {b:?}");
    }
    for h in w.hulls.iter().step_by(17) {
        let a=(h.bounds.min+h.bounds.max)*0.5;
        let b=a+Vec3::new(300.,-150.,60.);
        for half in [Vec3::ZERO,PLAYER_HALF] {
            let expected=original(w,a,b,half);let got=w.ledge_trace(a,b,half);
            assert_eq!((got.fraction,got.normal,got.start_solid,got.all_solid),
                       (expected.fraction,expected.normal,expected.start_solid,expected.all_solid));
        }
    }
}
#[test]
fn segment_broad_phase_preserves_collision_and_prunes_diagonal_queries() {
    let boxes=(0..32).flat_map(|x|(0..32).map(move |y| {
        let p=Vec3::new(x as f32*64.,y as f32*64.,0.);(p,p+Vec3::new(16.,16.,64.))
    })).collect::<Vec<_>>();
    let mut w=World::fixture(&boxes);w.min=Vec3::splat(-64.);w.max=Vec3::splat(2100.);
    compare(&w,10000);
    let q=SweepBounds::new(Vec3::new(-32.,-32.,20.),Vec3::new(2080.,2080.,20.),Vec3::splat(0.5));
    let mut old=0;let mut new=0;
    old_visit(w.tree.as_ref().unwrap(),q.bounds,&mut |_|old+=1);
    w.tree.as_ref().unwrap().visit(&q,&mut |_|new+=1);
    assert!(new*4<old,"candidates {old} -> {new}");
}
#[test]
#[ignore = "Requires LOOKING_GLASS_TEST_DATA pointing to the user's supplied PK3 directory"]
fn original_maze_and_clockwork_traces_match() {
    let mut a=crate::assets::Assets::open(&std::path::PathBuf::from(std::env::var("LOOKING_GLASS_TEST_DATA").unwrap())).unwrap();
    for name in ["hedge2","hedge3","hatter1"] {
        let map=Bsp::parse(&a.read(&format!("maps/{name}.bsp")).unwrap()).unwrap();
        for w in [World::from_bsp(&map).unwrap(),World::actor_world(&map).unwrap()] {compare(&w,10000);}
    }
}
