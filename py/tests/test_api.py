"""The Python API against the real engine (skipped when datum-sim-cli is not built or installed).
Run: DATUM_SIM_CLI=target/debug/datum-sim-cli python -m unittest discover -s py/tests"""
import math
import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.dirname(HERE))
import datum_sim as ds  # noqa: E402

try:
    CLI = ds.find_cli()
except ds.DatumSimError:
    CLI = None


@unittest.skipIf(CLI is None, "datum-sim-cli is not built")
class Api(unittest.TestCase):
    def test_run_and_closed_form(self):
        # The mass-spring-damper: x'' + 2 zeta wn x' + wn^2 x = F/m with a step at 0.5 s.
        with ds.example("msd") as m:
            m.set(zeta=0.2).settings(rtol=1e-9, atol=1e-11)
            r = m.run(stop=10)
            name = r.names[0]
            wn, z = math.sqrt(10.0), 0.2
            wd = wn * math.sqrt(1 - z * z)
            t = 3.0 - 0.5
            exact = 1 - math.exp(-z * wn * t) * (math.cos(wd * t) + z / math.sqrt(1 - z * z) * math.sin(wd * t))
            self.assertAlmostEqual(r.at(name, 3.0), exact, delta=1e-5)

    def test_params_sweep_save_and_errors(self):
        with ds.example("dcmotor") as m:
            self.assertIn("PID", m.blocks)
            m.set_param("PID", "P", 50)
            self.assertEqual(m.blocks["PID"]["params"]["P"], "50")
            runs = m.sweep("Kp", [50, 150], points=100)
            self.assertEqual([v for v, _ in runs], [50.0, 150.0])
            self.assertLessEqual(len(runs[0][1].t), 100)
            with self.assertRaises(ds.DatumSimError):
                m.set_param("Nope", "k", 1)
            with self.assertRaises(KeyError):
                m.run()["no such signal"]
            with tempfile.TemporaryDirectory() as d:
                p = os.path.join(d, "copy.dsim")
                m.save(p)
                with ds.open(p) as m2:
                    self.assertEqual(m2.blocks["PID"]["params"]["P"], "50")

    def test_examples_list(self):
        ids = [i for i, _ in ds.examples()]
        self.assertIn("msd", ids)
        self.assertIn("pmsmdigital", ids)


if __name__ == "__main__":
    unittest.main()
