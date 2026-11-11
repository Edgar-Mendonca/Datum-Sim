# Plan: robotics (stage R, 0.4.1 → 0.5.0)

Written 2026-11-11 (session 9). The user's choices: robotics comes **before** stage C (physical components), and **both** arms matter: a 6-joint industrial arm and a 7-joint arm. Status: **built and released 2026-11-11 (0.4.1 → 0.5.0)** with the defaults in §6 ("go ahead"). Record: `docs/verification/2026-11-11-robotics.md`.

## 1. Why, and where we start

The reference workflow for manipulator design has three layers:
1. A task scheduler with trajectories and inverse kinematics.
2. A model-based joint controller, tuned on a fast model of the arm's motion.
3. Verification on a full 3D model of the robot, with a gripper and contact.

0.3.6 has the smallest version of this:
- a two-link planar arm, its equations written by hand;
- trajectory, computed-torque and inverse-kinematics blocks;
- a 2D view.

Stage R makes it general: any serial arm, real robots loaded from description files, 3D, grippers and a task scheduler.

## 2. How it is built (the recommendation the user agreed to)

- **The robot's equations in Rust**, in `crates/core/src/multibody.rs`. They use the standard rigid-body algorithms for a chain of links:
  - recursive Newton–Euler for inverse dynamics;
  - composite rigid body for the mass matrix;
  - articulated body for forward dynamics.

  They are fast enough for live sliders and Loop Analysis, and tested against known answers. The two-link arm becomes one case of this.
- **Robots from URDF files** (the open robot-description format), read with a small XML library (MIT/Apache: checked and recorded in NOTICE before use). The two robots (§3) ship in the installer.
- **3D in the window** with three.js (MIT licence): the robot's own meshes, play and drag through time, the tool path, and the camera turned with the mouse.
- **Python stays around the engine, not in its loop.** It is for scripting and batch studies (`datum_sim`, as now), and later for offline work such as path planning or parameter fitting.

## 3. The two robots

| Arm | Joints | Candidate source | Licence (to verify before shipping) |
| --- | --- | --- | --- |
| 6-joint industrial arm | 6 | The robot maker's open ROS description package | BSD-3-Clause expected |
| 7-joint arm | 7 | The robot maker's open ROS description package | BSD-3-Clause expected (an Apache-2.0 alternative exists) |

- Each file's licence is checked, recorded in NOTICE and kept with the files.
- The app's public words name no products (FAMILY §13). The library says "6-joint industrial arm" and "7-joint arm", with the source and licence in NOTICE.
- If a licence does not allow shipping, the user loads that robot from its own URDF file instead.

## 4. Releases

| Release | What it adds | Checked by |
| --- | --- | --- |
| 0.4.1 | **Serial arm engine**: N joints (revolute and prismatic), link masses and inertias, joint friction. A **Robot arm (N joints)** block from a joint table. | Matches the two-link closed form; energy conserved; the mass matrix is symmetric and positive definite; inverse then forward dynamics returns the same accelerations |
| 0.4.2 | **URDF import** and the **robot library**, with the 6-joint and 7-joint arms | Joint count, limits and masses read back; gravity torques at a pose against an independent hand calculation for the 6-joint arm |
| 0.4.3 | **3D view**: meshes, play and drag through time, tool path, orbit camera | UI checks; the real app |
| 0.4.4 | **Inverse kinematics** for any arm (damped least squares, position and orientation, joint limits), and **tool-space trajectories** (straight lines and arcs) | The tip returned to its pose within 1e-9; the joint limits respected |
| 0.4.5 | **Controller blocks** for N joints: computed torque, PD + gravity, and blocks for the mass matrix, gravity and velocity torques. **Motors on joints**: our motor blocks drive joints through gears. | Tracking error with an exact model; the motor and gear torque balance |
| 0.4.6 | **Gripper and contact**: simple spring–damper contact with objects (a box on a table), grasp and release | An object at rest on the table; a box lifted and carried without slipping |
| 0.4.7 | **Task scheduler**: a small state-machine block (states, conditions, outputs), enough for pick and place. Full state machines stay in 0.7.x. | Sequences and timings in a test model |
| 0.4.8 | **Examples**: pick and place with the 6-joint arm and the 7-joint arm; tuning a joint loop with Control › Loop Analysis | All run; within tolerance of their targets |
| **0.5.0** | **Milestone**: verification record, docs | |

Later, with the planned Datum bench app: connecting the model to a real controller or robot.

## 5. What moves

Stage C (physical components) becomes 0.5.1 → 0.6.0. Exchange becomes 0.6.x, studies and logic 0.7.x, and reach and speed 0.8.x. The Roadmap page and the planned features in the window show the new numbers.

## 6. Questions for the user

1. **Installer size**: the two robots' 3D meshes add a few MB. Is that fine? (My default is yes, with the meshes simplified.)
2. **Contact**: simple spring–damper contact (enough to grasp and carry a box), not a full collision engine. Is that enough for now?
3. **State machine**: a small one inside stage R (0.4.7), with the full one staying in 0.7.x. Agreed?
4. **Releases**: one per step, as in stages A and B?
