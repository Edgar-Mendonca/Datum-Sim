# datum-sim for Python

Open, change, run and sweep Datum Sim models from Python and get the results as NumPy arrays.

```bash
pip install "git+https://github.com/Edgar-Mendonca/Datum-Sim#subdirectory=py"
```

```python
import datum_sim as ds
m = ds.open("motor.dsim")          # or ds.example("dcmotor")
m.set(Kp=120)                       # model variables
m.set_param("PID", "D", 5)          # a block parameter, by name or path ("Motor/Gain")
r = m.run(stop=5)                   # results
print(r.names, r.final(r.names[0]))
for value, r in m.sweep("Kp", [50, 100, 150]):
    print(value, r.final(r.names[0]))
```

It uses the `datum-sim-cli` program installed with the app (found through DATUM_SIM_CLI, the PATH or the app's folder).
