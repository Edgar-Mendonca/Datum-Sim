"""The worker protocol and the analyses, against closed-form answers. Run: python -m unittest discover -s py/tests"""
import json
import math
import os
import subprocess
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.dirname(HERE))
from datum_sim import linear  # noqa: E402


def second_order(wn, z):
    # x' = A x + B u, y = C x: wn² / (s² + 2 z wn s + wn²) in controllable form
    return {"A": [[-2 * z * wn, -wn * wn], [1, 0]], "B": [[1], [0]], "C": [[0, wn * wn]], "D": [[0]]}


class Analyses(unittest.TestCase):
    def facts(self, r):
        return dict(r["facts"])

    def test_second_order_step(self):
        r = linear.analyse({**second_order(2.0, 0.3), "what": "step"})
        f = self.facts(r)
        mp = 100 * math.exp(-0.3 * math.pi / math.sqrt(1 - 0.09))
        self.assertAlmostEqual(float(f["Overshoot"].split()[0]), mp, delta=0.5)
        self.assertEqual(f["Final value"], "1")
        self.assertEqual(f["Stable"], "yes")

    def test_bode_first_order(self):
        # 1/(s+1): −3 dB at 1 rad/s, phase −45° there; infinite gain margin
        r = linear.analyse({"A": [[-1]], "B": [[1]], "C": [[1]], "D": [[0]], "what": "bode"})
        w = r["w"]
        k = min(range(len(w)), key=lambda i: abs(w[i] - 1))
        self.assertAlmostEqual(r["mag_db"][k], -3.01, delta=0.05)
        self.assertAlmostEqual(r["phase_deg"][k], -45, delta=0.5)
        self.assertIn("Bandwidth (−3 dB)", self.facts(r))

    def test_poles_and_zeros(self):
        r = linear.analyse({**second_order(2.0, 0.5), "what": "pz"})
        re = sorted(p[0] for p in r["poles"])
        self.assertAlmostEqual(re[0], -1.0, places=6)
        self.assertAlmostEqual(abs(r["poles"][0][1]), math.sqrt(3), places=6)

    def test_unstable_step_is_refused(self):
        with self.assertRaises(ValueError):
            linear.analyse({"A": [[1]], "B": [[1]], "C": [[1]], "D": [[0]], "what": "step"})

    def test_step_info_from_data(self):
        t = [i * 0.01 for i in range(1001)]
        y = [1 - math.exp(-x) for x in t]
        f = dict(linear.step_info({"t": t, "y": y})["facts"])
        self.assertAlmostEqual(float(f["Rise time (10–90 %)"].split()[0]), math.log(9), delta=0.02)


class Protocol(unittest.TestCase):
    def test_lines_with_ids(self):
        p = subprocess.Popen([sys.executable, "-X", "utf8", os.path.join(os.path.dirname(HERE), "worker.py")], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        try:
            self.assertTrue(json.loads(p.stdout.readline())["ready"])
            for i, q in enumerate([{"cmd": "ping"}, {"cmd": "nope"}, {"cmd": "linear", "what": "bode", "A": [[-1]], "B": [[1]], "C": [[1]], "D": [[0]]}]):
                p.stdin.write(json.dumps({**q, "id": i + 1}) + "\n")
                p.stdin.flush()
                r = json.loads(p.stdout.readline())
                self.assertEqual(r["id"], i + 1)
                self.assertEqual(r["ok"], q["cmd"] != "nope")
            p.stdin.write("this is not json\n")
            p.stdin.flush()
            self.assertFalse(json.loads(p.stdout.readline())["ok"])
        finally:
            p.kill()
            p.wait()
            p.stdin.close()
            p.stdout.close()


if __name__ == "__main__":
    unittest.main()
