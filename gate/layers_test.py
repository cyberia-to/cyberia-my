#!/usr/bin/env python3
"""tests of the map's acts and their gate: python3 gate/layers_test.py"""
import subprocess, sys, tempfile, time, unittest, uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import layers, sign  # noqa: E402

OWNER_K, EDITOR_K, GUEST_K, OTHER_K = 11, 22, 33, 44
OWNER, EDITOR, GUEST, OTHER = (sign.address_of(k) for k in (OWNER_K, EDITOR_K, GUEST_K, OTHER_K))
LAYER = str(uuid.UUID(int=900))
RING = [[1150890444, -82973027], [1150886763, -82977116], [1150887313, -82980103], [1150891768, -82978177]]


def feat(n, kind="plot", geom=None, prev="", **props):
    return {"id": str(uuid.UUID(int=1000 + n)), "kind": kind, "geom": geom or RING, "props": props, "prev": prev}


class Atlas:
    """a map in memory: sign an act with a key, fold it; files are kept by name."""
    def __init__(self):
        self.state = layers.empty(); self.n = 0; self.files = {}

    def act(self, key, kind, **f):
        self.n += 1
        fields = {"kind": kind, "id": str(uuid.UUID(int=self.n)), "at": f"2026-11-21T10:{self.n // 60:02d}:{self.n % 60:02d}Z", "layer": LAYER, **f}
        message = layers.render(fields)
        who = sign.recover(message, sign.sign(message, key))
        layers.apply(self.state, layers.fields(message), who, self.files.get)
        return fields["id"]

    def change(self, key, put=(), remove=(), **f):
        data = layers.canon({"layer": LAYER, "put": list(put), "remove": list(remove)})
        name = layers.name_of(data); self.files[name] = data
        return self.act(key, "change", file=name, put=len(put), remove=len(remove), **f)

    def head(self, n): return self.state["layers"][LAYER]["features"][str(uuid.UUID(int=1000 + n))]


class LayersTest(unittest.TestCase):
    def setUp(self):
        self.m = Atlas()
        self.m.change(OWNER_K, put=[feat(1, name="sinwood east")], name="cyber valley")

    def test_the_first_change_founds_the_layer_and_its_signer_owns_it(self):
        l = self.m.state["layers"][LAYER]
        self.assertEqual((l["owner"], l["name"], len(l["features"]), l["public"], l["open"]), (OWNER, "cyber valley", 1, True, False))
        with self.assertRaises(ValueError): Atlas().change(OWNER_K, put=[feat(1)])   # a new layer has a name

    def test_only_the_owner_and_editors_change_a_closed_layer(self):
        with self.assertRaises(ValueError): self.m.change(GUEST_K, put=[feat(2)])
        with self.assertRaises(ValueError): self.m.act(GUEST_K, "grant", address=GUEST, role="editor")
        self.m.act(OWNER_K, "grant", address=EDITOR, role="editor")
        self.m.change(EDITOR_K, put=[feat(2, kind="point", geom=[RING[0]], name="gate")])
        self.m.act(OWNER_K, "grant", address=EDITOR, role="none")
        with self.assertRaises(ValueError): self.m.change(EDITOR_K, put=[feat(3)])

    def test_an_edit_names_the_version_it_replaces(self):
        v1 = self.m.head(1)["version"]
        with self.assertRaises(ValueError): self.m.change(OWNER_K, put=[feat(1, name="moved")])                 # no prev
        self.m.change(OWNER_K, put=[feat(1, prev=v1, name="sinwood east", note="resurveyed")])
        with self.assertRaises(ValueError): self.m.change(OWNER_K, put=[feat(1, prev=v1, name="stale")])     # v1 is no longer the head
        x = self.m.head(1)
        self.assertEqual((len(x["history"]), x["props"]["note"], x["by"]), (2, "resurveyed", OWNER))
        self.assertEqual(x["version"], layers.version(feat(1, name="sinwood east", note="resurveyed")))

    def test_a_removed_feature_is_gone_and_does_not_come_back(self):
        self.m.change(OWNER_K, remove=[feat(1)["id"]])
        self.assertEqual(self.m.state["layers"][LAYER]["features"], {})
        with self.assertRaises(ValueError): self.m.change(OWNER_K, put=[feat(1)])
        with self.assertRaises(ValueError): self.m.change(OWNER_K, remove=[feat(1)["id"]])

    def test_on_an_open_layer_a_guest_touches_only_their_own(self):
        self.m.act(OWNER_K, "layer", open="yes")
        self.m.change(GUEST_K, put=[feat(5, kind="point", geom=[RING[1]], name="spring")])
        with self.assertRaises(ValueError): self.m.change(GUEST_K, put=[feat(1, prev=self.m.head(1)["version"], name="mine now")])
        with self.assertRaises(ValueError): self.m.change(GUEST_K, remove=[feat(1)["id"]])
        with self.assertRaises(ValueError): self.m.change(OTHER_K, remove=[feat(5)["id"]])
        self.m.change(GUEST_K, remove=[feat(5)["id"]])

    def test_a_feature_is_whole_numbers_within_the_globe(self):
        bad = [feat(2, geom=[[1.5, 2]] * 3), feat(2, geom=RING[:2]), feat(2, kind="point"), feat(2, kind="tower"),
               feat(2, geom=[[1900000000, 0]] * 3), {**feat(2), "props": {"Bad Key": "x"}}, {**feat(2), "extra": 1}, {**feat(2), "props": {"h": 1.5}}]
        for f in bad:
            with self.assertRaises(ValueError, msg=str(f)[:80]): self.m.change(OWNER_K, put=[f])
        self.m.change(OWNER_K, put=[feat(2, kind="building", height_cm=900, base_cm=0, levels=3), feat(3, kind="line", geom=RING[:2])])

    def test_the_act_names_the_file_and_the_file_is_canonical(self):
        data = b'{"layer": "%s", "put": [], "remove": []}' % LAYER.encode()
        self.m.files[layers.name_of(data)] = data
        with self.assertRaises(ValueError): self.m.act(OWNER_K, "change", file=layers.name_of(data), put=0, remove=0)          # spaces: not canonical
        with self.assertRaises(ValueError): self.m.act(OWNER_K, "change", file="sha256:" + "0" * 64, put=1, remove=0)           # no such file
        good = layers.canon({"layer": LAYER, "put": [feat(2)], "remove": []}); self.m.files[layers.name_of(good)] = good
        with self.assertRaises(ValueError): self.m.act(OWNER_K, "change", file=layers.name_of(good), put=7, remove=0)           # the wallet was shown another count

    def test_a_private_layer_is_seen_by_its_people_only(self):
        self.m.act(OWNER_K, "layer", public="no", name="the survey")
        self.assertEqual(layers.layers(self.m.state, None), [])
        self.assertIsNone(layers.layer(self.m.state, LAYER, GUEST))
        self.assertEqual(layers.layer(self.m.state, LAYER, OWNER)["name"], "the survey")
        with self.assertRaises(ValueError): self.m.act(GUEST_K, "attest", feature=feat(1)["id"], version=self.m.head(1)["version"], statement="confirm", method="manual")

    def test_a_statement_on_the_spot_carries_its_position_and_holds_for_one_version(self):
        fid, v = feat(1)["id"], self.m.head(1)["version"]
        with self.assertRaises(ValueError): self.m.act(GUEST_K, "attest", feature=fid, version=v, statement="confirm")                       # on the spot, no position
        with self.assertRaises(ValueError): self.m.act(GUEST_K, "attest", feature=fid, version="sha256:" + "1" * 64, statement="confirm", method="manual")
        with self.assertRaises(ValueError): self.m.act(GUEST_K, "attest", feature=fid, version=v, statement="confirm", lat_e7=RING[0][1], lon_e7=RING[0][0], accuracy_cm=400, vertex=9)
        self.m.act(GUEST_K, "attest", feature=fid, version=v, statement="confirm", role="neighbour", lat_e7=RING[0][1] + 90, lon_e7=RING[0][0], accuracy_cm=400, samples=8, spread_cm=120, vertex=0)
        self.m.act(OTHER_K, "attest", feature=fid, version=v, statement="dispute", method="manual", note="the fence stands further east")
        a = self.m.head(1)["attests"][0]
        self.assertEqual((a["by"], a["vertex"], a["distance_cm"], a["role"]), (GUEST, 0, 100, "neighbour"))   # 90 e7 of latitude is a metre
        self.assertEqual(layers.verified(self.m.head(1)), {"confirm": 1, "dispute": 1, "on_spot": 1})
        # the same signer speaks again of the same corner: one word stays
        self.m.act(GUEST_K, "attest", feature=fid, version=v, statement="confirm", lat_e7=RING[0][1], lon_e7=RING[0][0], accuracy_cm=300, vertex=0)
        self.assertEqual(len(self.m.head(1)["attests"]), 2)
        # the boundary moves: what was signed about the old one no longer stands
        self.m.change(OWNER_K, put=[feat(1, prev=v, geom=RING[:3], name="sinwood east")])
        self.assertEqual(self.m.head(1)["attests"], [])

    def test_the_gate_commits_the_act_and_its_file_and_the_fold_reads_them_back(self):
        import gate
        with tempfile.TemporaryDirectory() as d:
            subprocess.run(["git", "-C", d, "init", "-q", "-b", "main"], check=True)
            g = gate.Gate(d, OWNER, b"secret", None, push=False)
            data = layers.canon({"layer": LAYER, "put": [feat(1, name="a")], "remove": []})
            now = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
            msg = layers.render({"kind": "change", "id": str(uuid.UUID(int=1)), "at": now, "layer": LAYER, "file": layers.name_of(data), "put": 1, "remove": 0, "name": "bench"})
            with self.assertRaises(ValueError): g.map_act(msg, sign.sign(msg, OWNER_K), data.decode() + " ")   # not the named bytes
            out = g.map_act(msg, sign.sign(msg, OWNER_K), data.decode())
            self.assertEqual((out["layer"]["name"], len(out["layer"]["items"]), out["layer"]["mine"]), ("bench", 1, True))
            with self.assertRaises(ValueError): g.map_act(msg, sign.sign(msg, OWNER_K), data.decode())          # the id is used
            st, errs = layers.fold(Path(d) / "map")
            self.assertEqual((errs, len(st["layers"][LAYER]["features"])), ([], 1))
            # a file put beside the acts by hand under a wrong name is named by the fold
            layers.file_path(Path(d) / "map", layers.name_of(data)).write_bytes(data + b" ")
            self.assertEqual(len(layers.fold(Path(d) / "map")[1]), 1)


if __name__ == "__main__":
    unittest.main()
