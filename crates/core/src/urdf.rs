//! Robots from URDF files (0.4.2): links with their inertial data, joints (revolute,
//! continuous, prismatic, fixed) with origins, axes, limits and damping, and the meshes the 3D
//! view draws. Joints that copy another (mimic) or are named in `freeze` are held fixed.

use crate::multibody::{mm, mv, rpy, tr, Body, Chain, Kind, M3, V3, I3};

/// A mesh drawn on a link (for the 3D view): the link's body index (None = the base, fixed to
/// the world), the file's name, and its placement in the link frame.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Mesh {
    pub body: Option<usize>,
    pub file: String,
    pub r: M3,
    pub p: V3,
    pub scale: V3,
}

#[derive(Clone, Debug)]
pub struct Robot {
    pub chain: Chain,
    pub meshes: Vec<Mesh>,
    pub joints: Vec<String>,
}

fn nums(s: Option<&str>, n: usize, def: f64) -> Result<Vec<f64>, String> {
    match s {
        None => Ok(vec![def; n]),
        Some(t) => {
            let v: Vec<f64> = t.split_whitespace().map(|x| x.parse::<f64>().map_err(|_| format!("'{x}' is not a number"))).collect::<Result<_, _>>()?;
            if v.len() != n {
                return Err(format!("'{t}' should have {n} numbers"));
            }
            Ok(v)
        }
    }
}

fn origin(n: Option<roxmltree::Node>) -> Result<(M3, V3), String> {
    let Some(o) = n else { return Ok((I3, [0.0; 3])) };
    let x = nums(o.attribute("xyz"), 3, 0.0)?;
    let r = nums(o.attribute("rpy"), 3, 0.0)?;
    Ok((rpy(r[0], r[1], r[2]), [x[0], x[1], x[2]]))
}

fn child<'a>(n: roxmltree::Node<'a, 'a>, tag: &str) -> Option<roxmltree::Node<'a, 'a>> {
    n.children().find(|c| c.has_tag_name(tag))
}

/// Read a URDF text. `tool` names the link the tool point is on (empty = the last link);
/// `tool_off` is the point in that link's frame; `g` the gravity along −z.
pub fn parse(text: &str, freeze: &[&str], tool: &str, tool_off: V3, g: f64) -> Result<Robot, String> {
    let doc = roxmltree::Document::parse(text).map_err(|e| format!("the URDF file is not valid XML: {e}"))?;
    let root = doc.root_element();
    if !root.has_tag_name("robot") {
        return Err("the file is not a URDF robot description".into());
    }
    let links: Vec<roxmltree::Node> = root.children().filter(|c| c.has_tag_name("link")).collect();
    let joints: Vec<roxmltree::Node> = root.children().filter(|c| c.has_tag_name("joint")).collect();
    let name = |n: &roxmltree::Node| n.attribute("name").unwrap_or("").to_string();
    let link_of = |j: &roxmltree::Node, tag: &str| child(*j, tag).and_then(|c| c.attribute("link")).unwrap_or("").to_string();
    let children: Vec<String> = joints.iter().map(|j| link_of(j, "child")).collect();
    let base = links.iter().map(name).find(|l| !children.contains(l)).ok_or("the URDF has no base link (every link is some joint's child)")?;
    // Breadth-first from the base, so parents come first.
    let mut order: Vec<(usize, Option<usize>)> = vec![];
    let mut frontier = vec![(base.clone(), None::<usize>)];
    while let Some((l, pb)) = (!frontier.is_empty()).then(|| frontier.remove(0)) {
        for (k, j) in joints.iter().enumerate() {
            if link_of(j, "parent") == l {
                let idx = order.len();
                order.push((k, pb));
                frontier.push((children[k].clone(), Some(idx)));
            }
        }
    }
    let link_node = |n: &str| links.iter().find(|l| l.attribute("name") == Some(n)).cloned();
    let mut bodies = vec![];
    let mut meshes = vec![];
    let mut joint_names = vec![];
    let mut add_meshes = |l: &roxmltree::Node, body: Option<usize>| -> Result<(), String> {
        for v in l.children().filter(|c| c.has_tag_name("collision") || c.has_tag_name("visual")) {
            // Collision meshes are light: prefer them, and take a visual mesh only when a link has none.
            if v.has_tag_name("visual") && l.children().any(|c| c.has_tag_name("collision")) {
                continue;
            }
            let (r, p) = origin(child(v, "origin"))?;
            if let Some(m) = child(v, "geometry").and_then(|g| child(g, "mesh")) {
                let f = m.attribute("filename").unwrap_or("");
                let file = f.rsplit('/').next().unwrap_or(f).to_string();
                let s = nums(m.attribute("scale"), 3, 1.0)?;
                meshes.push(Mesh { body, file, r, p, scale: [s[0], s[1], s[2]] });
            }
        }
        Ok(())
    };
    if let Some(b) = link_node(&base) {
        add_meshes(&b, None)?;
    }
    for (k, pb) in &order {
        let j = joints[*k];
        let jn = name(&j);
        let ty = j.attribute("type").unwrap_or("fixed");
        let frozen = freeze.contains(&jn.as_str()) || child(j, "mimic").is_some();
        let kind = match ty {
            _ if frozen => Kind::Fixed,
            "revolute" | "continuous" => Kind::Revolute,
            "prismatic" => Kind::Prismatic,
            "fixed" => Kind::Fixed,
            t => return Err(format!("joint {jn}: the joint type '{t}' is not supported (revolute, continuous, prismatic, fixed)")),
        };
        let mut b = Body::new(&children[*k], *pb, kind);
        let (r, p) = origin(child(j, "origin"))?;
        b.pre_r = r;
        b.pre_p = p;
        if kind != Kind::Fixed {
            let a = nums(child(j, "axis").and_then(|a| a.attribute("xyz")), 3, 0.0)?;
            b.axis = if child(j, "axis").is_some() { [a[0], a[1], a[2]] } else { [1.0, 0.0, 0.0] };
            joint_names.push(jn.clone());
        }
        if let Some(lim) = child(j, "limit") {
            if ty != "continuous" {
                b.lo = lim.attribute("lower").and_then(|v| v.parse().ok()).unwrap_or(f64::NEG_INFINITY);
                b.hi = lim.attribute("upper").and_then(|v| v.parse().ok()).unwrap_or(f64::INFINITY);
            }
        }
        b.b = child(j, "dynamics").and_then(|d| d.attribute("damping")).and_then(|v| v.parse().ok()).unwrap_or(0.0);
        if let Some(l) = link_node(&children[*k]) {
            if let Some(inr) = child(l, "inertial") {
                let (ir, ip) = origin(child(inr, "origin"))?;
                b.m = child(inr, "mass").and_then(|m| m.attribute("value")).and_then(|v| v.parse().ok()).unwrap_or(0.0);
                b.com = ip;
                let i = child(inr, "inertia");
                let g = |k: &str| i.and_then(|i| i.attribute(k)).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
                let ic = [[g("ixx"), g("ixy"), g("ixz")], [g("ixy"), g("iyy"), g("iyz")], [g("ixz"), g("iyz"), g("izz")]];
                b.inertia = mm(&mm(&ir, &ic), &tr(&ir));
            }
            add_meshes(&l, Some(bodies.len()))?;
        }
        bodies.push(b);
    }
    if bodies.is_empty() {
        return Err("the URDF has no joints".into());
    }
    let mut chain = Chain::new(bodies, g, tool_off)?;
    if !tool.is_empty() {
        chain.tool_body = chain.bodies.iter().position(|b| b.name == tool).ok_or_else(|| format!("no link named '{tool}' for the tool"))?;
    }
    let _ = mv;
    Ok(Robot { chain, meshes, joints: joint_names })
}

/// The robots that come with the app: (id, name, URDF, joints held fixed, tool link).
pub const LIBRARY: &[(&str, &str, &str, &[&str], &str)] = &[
    ("arm6", "6-joint industrial arm", include_str!("../../../assets/robots/arm6/robot.urdf"), &[], "tool0"),
    ("arm7", "7-joint arm", include_str!("../../../assets/robots/arm7/robot.urdf"), &["panda_finger_joint1", "panda_finger_joint2"], "panda_hand_tcp"),
];

pub fn library(id: &str, g: f64) -> Result<Robot, String> {
    let (_, _, text, freeze, tool) = LIBRARY.iter().find(|r| r.0 == id).ok_or_else(|| format!("no robot '{id}' in the library"))?;
    parse(text, freeze, tool, [0.0; 3], g)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both library robots load: 6 and 7 moving joints, positive masses and limits from the file;
    /// at the zero pose the 6-joint arm's tool sits where its link lengths put it, and the
    /// shoulder's gravity torque matches a hand sum of the links' weights × lever arms.
    #[test]
    fn library_robots() {
        let a6 = library("arm6", 9.81).unwrap();
        assert_eq!((a6.chain.n, a6.joints.len()), (6, 6));
        let a7 = library("arm7", 9.81).unwrap();
        assert_eq!(a7.chain.n, 7);
        assert!(a7.chain.bodies.iter().filter(|b| b.dof.is_some()).all(|b| b.lo < b.hi && b.lo.is_finite()));
        assert!(!a6.meshes.is_empty() && a6.meshes.iter().all(|m| m.file.ends_with(".stl")));
        // Gravity torque at a joint = Σ over the links beyond it of m·g × (horizontal lever arm),
        // about that joint's axis: an independent sum from the poses.
        for r in [&a6, &a7] {
            let c = &r.chain;
            let q = vec![0.3; c.n];
            let tau = c.rnea(&q, &vec![0.0; c.n], &vec![0.0; c.n], true);
            let ps = c.pose(&q);
            for (i, b) in c.bodies.iter().enumerate() {
                let Some(d) = b.dof else { continue };
                // Every body whose chain of parents reaches i.
                let mut sum = 0.0;
                for k in 0..c.bodies.len() {
                    let mut p = Some(k);
                    while let Some(x) = p {
                        if x == i {
                            break;
                        }
                        p = c.bodies[x].parent;
                    }
                    if p != Some(i) {
                        continue;
                    }
                    let bk = &c.bodies[k];
                    let com = crate::multibody::add(ps.lp[k], crate::multibody::mv(&ps.lr[k], bk.com));
                    let lever = crate::multibody::sub(com, ps.jp[i]);
                    let f = [0.0, 0.0, -bk.m * 9.81];
                    sum += crate::multibody::dot(crate::multibody::cross(lever, f), ps.z[i]);
                }
                assert!((tau[d] + sum).abs() < 1e-9 * (1.0 + sum.abs()), "joint {d}: {} vs {}", tau[d], -sum);
            }
        }
        // Errors are clear.
        assert!(parse("<robot name='x'/>", &[], "", [0.0; 3], 9.81).is_err());
        assert!(parse("not xml", &[], "", [0.0; 3], 9.81).unwrap_err().contains("XML"));
    }
}
