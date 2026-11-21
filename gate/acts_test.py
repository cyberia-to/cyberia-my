#!/usr/bin/env python3
"""tests of the acts and the gate: python3 gate/acts_test.py"""
import json, subprocess, sys, tempfile, unittest, uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import acts, sign  # noqa: E402

ROOT_K, ADMIN_K, HOLDER_K, AGENT_K, STRANGER_K = 11, 22, 33, 44, 55
ROOT, ADMIN, HOLDER, AGENT, STRANGER = (sign.address_of(k) for k in (ROOT_K, ADMIN_K, HOLDER_K, AGENT_K, STRANGER_K))


class Market:
    """a market in memory: sign an act with a key, fold it."""
    def __init__(self):
        self.state = acts.empty(); self.n = 0

    def act(self, key, kind, **f):
        self.n += 1
        at = f"2026-11-20T10:{self.n // 60:02d}:{self.n % 60:02d}Z"
        fields = {"kind": kind, "id": str(uuid.UUID(int=self.n)), "at": at, **f}
        message = acts.render(fields)
        who = sign.recover(message, sign.sign(message, key))
        acts.apply(self.state, acts.fields(message), who, ROOT)
        return fields["id"]


class SignTest(unittest.TestCase):
    def test_keccak_is_the_fips_permutation(self):
        import hashlib
        for n in (0, 1, 135, 136, 137, 500):
            data = bytes(range(256)) * 2
            self.assertEqual(sign.sponge(data[:n], pad=0x06), hashlib.sha3_256(data[:n]).digest())

    def test_a_known_key_has_its_known_address(self):
        # the web3.js documentation's key
        self.assertEqual(sign.address_of(0x4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318), "0x2c7536E3605D9C16a7a3D7b1898e529396a65c23")

    def test_recover_returns_the_signer_and_a_changed_message_another(self):
        s = sign.sign("the act", HOLDER_K)
        self.assertEqual(sign.recover("the act", s), HOLDER)
        self.assertNotEqual(sign.recover("the act.", s), HOLDER)


class ActsTest(unittest.TestCase):
    def setUp(self):
        self.m = Market()
        self.m.act(ROOT_K, "role", address=ADMIN, role="admin", name="valley desk")
        self.m.act(ADMIN_K, "holder", plot="sinwood-7", address=HOLDER, name="Ana")

    def test_only_the_root_names_admins_and_only_admins_bind_holders(self):
        with self.assertRaises(ValueError): self.m.act(ADMIN_K, "role", address=STRANGER, role="admin")
        with self.assertRaises(ValueError): self.m.act(STRANGER_K, "holder", plot="sinwood-8", address=STRANGER)
        self.m.act(ADMIN_K, "role", address=AGENT, role="agent", name="Bo")   # an admin accredits an agent
        with self.assertRaises(ValueError): self.m.act(ADMIN_K, "role", address=ADMIN, role="none")   # nor removes an admin

    def test_the_holder_lists_and_the_valley_approves(self):
        lid = self.m.act(HOLDER_K, "listing", plot="sinwood-7", deal="sale", price_usd="42000", words="by the spring", contact="@ana")
        self.assertEqual(acts.public(self.m.state)["listings"], [])   # pending is not public
        with self.assertRaises(ValueError): self.m.act(HOLDER_K, "approve", listing=lid)
        self.m.act(ADMIN_K, "approve", listing=lid)
        pub = acts.public(self.m.state)["listings"]
        self.assertEqual((pub[0]["plot"], pub[0]["price_usd"], pub[0]["seller"]["as"], pub[0]["seller"]["name"]), ("sinwood-7", 42000.0, "holder", "Ana"))
        self.assertNotIn(HOLDER, json.dumps(pub))   # no address leaves the gate

    def test_a_stranger_and_an_agent_without_a_mandate_cannot_list(self):
        for k in (STRANGER_K, AGENT_K):
            with self.assertRaises(ValueError): self.m.act(k, "listing", plot="sinwood-7", deal="sale", price_usd="1")
        with self.assertRaises(ValueError): self.m.act(HOLDER_K, "listing", plot="sinwood-9", deal="sale", price_usd="1")   # no holder bound

    def test_an_agent_lists_under_a_mandate_and_only_within_it(self):
        with self.assertRaises(ValueError): self.m.act(STRANGER_K, "mandate", plot="sinwood-7", agent=AGENT, scope="sale", until="2027-01-01")
        mid = self.m.act(HOLDER_K, "mandate", plot="sinwood-7", agent=AGENT, scope="lease", commission="5", until="2027-01-01")
        with self.assertRaises(ValueError): self.m.act(AGENT_K, "listing", plot="sinwood-7", deal="sale", price_usd="42000")   # the mandate is for lease
        self.m.act(AGENT_K, "profile", name="Bo of Bali Land", contact="@bo")
        lid = self.m.act(AGENT_K, "listing", plot="sinwood-7", deal="lease", price_usd="3000", rent_usd_year="3000", term_years="5")
        self.m.act(ADMIN_K, "approve", listing=lid)
        s = acts.public(self.m.state)["listings"][0]["seller"]
        self.assertEqual((s["as"], s["name"], s["contact"], s["accredited"]), ("agent", "Bo of Bali Land", "@bo", False))
        self.assertEqual(self.m.state["listings"][lid]["mandate"], mid)

    def test_a_revoked_mandate_takes_the_agents_listing_down(self):
        mid = self.m.act(HOLDER_K, "mandate", plot="sinwood-7", agent=AGENT, scope="both", until="2027-01-01")
        lid = self.m.act(AGENT_K, "listing", plot="sinwood-7", deal="sale", price_usd="42000")
        self.m.act(ADMIN_K, "approve", listing=lid)
        self.m.act(HOLDER_K, "revoke", mandate=mid)
        self.assertEqual(self.m.state["listings"][lid]["status"], "withdrawn")
        with self.assertRaises(ValueError): self.m.act(AGENT_K, "listing", plot="sinwood-7", deal="sale", price_usd="42000")

    def test_an_expired_mandate_lists_nothing(self):
        self.m.act(HOLDER_K, "mandate", plot="sinwood-7", agent=AGENT, scope="both", until="2026-11-20")
        self.m.act(AGENT_K, "listing", plot="sinwood-7", deal="sale", price_usd="1")   # the last day still counts
        st = self.m.state
        f = {"kind": "listing", "id": str(uuid.UUID(int=999)), "at": "2026-11-21T00:00:01Z", "plot": "sinwood-7", "deal": "sale", "price_usd": "1"}
        with self.assertRaises(ValueError): acts.apply(st, f, AGENT, ROOT)

    def test_a_new_offer_takes_the_old_ones_place_and_a_withdrawal_ends_it(self):
        a = self.m.act(HOLDER_K, "listing", plot="sinwood-7", deal="sale", price_usd="42000")
        b = self.m.act(HOLDER_K, "listing", plot="sinwood-7", deal="sale", price_usd="40000")
        self.assertEqual((self.m.state["listings"][a]["status"], self.m.state["listings"][b]["status"]), ("superseded", "pending"))
        with self.assertRaises(ValueError): self.m.act(STRANGER_K, "withdraw", listing=b)
        self.m.act(HOLDER_K, "withdraw", listing=b)
        self.assertEqual(self.m.state["listings"][b]["status"], "withdrawn")

    def test_a_flat_that_changes_hands_drops_its_mandates_and_offers(self):
        mid = self.m.act(HOLDER_K, "mandate", plot="sinwood-7", agent=AGENT, scope="both", until="2027-01-01")
        lid = self.m.act(HOLDER_K, "listing", plot="sinwood-7", deal="sale", price_usd="42000")
        self.m.act(ADMIN_K, "holder", plot="sinwood-7", address=STRANGER, name="Cy")
        self.assertEqual((self.m.state["mandates"][mid]["status"], self.m.state["listings"][lid]["status"]), ("ended", "withdrawn"))
        with self.assertRaises(ValueError): self.m.act(HOLDER_K, "listing", plot="sinwood-7", deal="sale", price_usd="1")

    def test_a_claim_is_answered_by_binding_or_rejecting(self):
        c = self.m.act(STRANGER_K, "claim", plot="sinwood-9", name="Cy", contact="@cy")
        self.assertEqual(self.m.state["claims"][c]["status"], "pending")
        self.m.act(ADMIN_K, "holder", plot="sinwood-9", address=STRANGER, name="Cy")
        self.assertEqual(self.m.state["claims"][c]["status"], "bound")
        c2 = self.m.act(AGENT_K, "claim", plot="sinwood-10", name="Bo")
        self.m.act(ADMIN_K, "reject", target=c2, reason="not in the lease book")
        self.assertEqual(self.m.state["claims"][c2]["status"], "rejected")

    def test_only_a_flat_of_the_valley_is_claimed_or_bound(self):
        acts.FLATS = {"sinwood-7", "sinwood-8"}
        try:
            with self.assertRaises(ValueError): self.m.act(STRANGER_K, "claim", plot="sinwood-1", name="Cy")
            with self.assertRaises(ValueError): self.m.act(ADMIN_K, "holder", plot="moon-1", address=STRANGER)
            self.m.act(STRANGER_K, "claim", plot="sinwood-8", name="Cy")
        finally:
            acts.FLATS = None

    def test_an_id_is_used_once_and_a_view_is_ones_own(self):
        lid = self.m.act(HOLDER_K, "listing", plot="sinwood-7", deal="sale", price_usd="42000")
        f = {"kind": "withdraw", "id": lid, "at": "2026-11-20T11:00:00Z", "listing": lid}
        with self.assertRaises(ValueError): acts.apply(self.m.state, f, HOLDER, ROOT)
        self.assertEqual(len(acts.view(self.m.state, STRANGER, ROOT)["listings"]), 0)
        self.assertEqual(len(acts.view(self.m.state, HOLDER, ROOT)["listings"]), 1)
        self.assertEqual(len(acts.view(self.m.state, ADMIN, ROOT)["listings"]), 1)


class DealsTest(unittest.TestCase):
    def setUp(self):
        self.m = Market()
        self.m.act(ROOT_K, "role", address=ADMIN, role="admin", name="valley desk")

    def test_only_an_admin_writes_a_flats_deal(self):
        for kind, f in (("deal", dict(status="reserved")), ("payment", dict(usd="100")), ("step", dict(step="kyc"))):
            with self.assertRaises(ValueError): self.m.act(HOLDER_K, kind, plot="sinwood-7", **f)
        self.assertEqual(self.m.state["deals"], {})

    def test_the_first_money_reserves_the_flat_and_payments_add_up(self):
        self.m.act(ADMIN_K, "payment", plot="sinwood-7", usd="7950", method="bank", reference="LEASE SINWOOD-7 PREMIUM 30%", on="2026-11-18")
        d = acts.public_deals(self.m.state)["sinwood-7"]
        self.assertEqual((d["status"], d["paid_usd"], d["steps"]["premium"], d["steps"]["request"]), ("reserved", 7950.0, "2026-11-18", "2026-11-18"))
        self.m.act(ADMIN_K, "payment", plot="sinwood-7", usd="18550.50", method="eth")
        d = acts.public_deals(self.m.state)["sinwood-7"]
        self.assertEqual((d["paid_usd"], len(d["payments"])), (26500.5, 2))
        with self.assertRaises(ValueError): self.m.act(ADMIN_K, "payment", plot="sinwood-7", usd="0")
        with self.assertRaises(ValueError): self.m.act(ADMIN_K, "payment", plot="sinwood-7", usd="5", method="cash")

    def test_terms_steps_and_the_signature_that_makes_it_owned(self):
        self.m.act(ADMIN_K, "deal", plot="sinwood-7", holder="Ana", form="index", premium_pct="30", price_usd="26500")
        self.m.act(ADMIN_K, "step", plot="sinwood-7", step="kyc", on="2026-11-19")
        d = acts.public_deals(self.m.state)["sinwood-7"]
        self.assertEqual((d["status"], d["holder"], d["form"], d["premium_pct"], d["price_usd"], d["steps"]["kyc"]), ("reserved", "Ana", "index", 30, 26500.0, "2026-11-19"))
        with self.assertRaises(ValueError): self.m.act(ADMIN_K, "step", plot="sinwood-7", step="dance")
        with self.assertRaises(ValueError): self.m.act(ADMIN_K, "deal", plot="sinwood-7", status="sold")
        self.m.act(ADMIN_K, "step", plot="sinwood-7", step="signed")
        self.assertEqual(acts.public_deals(self.m.state)["sinwood-7"]["status"], "owned")

    def test_available_closes_the_deal_and_the_trail_keeps_it(self):
        self.m.act(ADMIN_K, "deal", plot="sinwood-7", status="transfer", holder="Ana")
        self.m.act(ADMIN_K, "deal", plot="sinwood-7", status="available")
        self.assertEqual(acts.public_deals(self.m.state), {})
        log = acts.view(self.m.state, ADMIN, ROOT)["log"]
        self.assertEqual([(l["kind"], l["what"]) for l in log[:2]], [("deal", "sinwood-7"), ("deal", "sinwood-7")])
        self.assertNotIn("log", acts.view(self.m.state, HOLDER, ROOT))   # the trail is the desk's


class FilesTest(unittest.TestCase):
    def test_files_fold_to_the_same_state_and_a_forged_file_is_named(self):
        with tempfile.TemporaryDirectory() as d:
            def put(n, key, **f):
                fields = {"id": str(uuid.UUID(int=n)), "at": f"2026-11-20T10:00:{n:02d}Z", **f}
                message = acts.render(fields)
                sig = sign.sign(message, key)
                Path(d, f"{n:02d}.md").write_text(acts.file_text(message, sig, sign.recover(message, sig)))
            put(1, ADMIN_K, kind="holder", plot="sinwood-7", address=HOLDER)      # the admin is not named yet: refused
            put(2, ROOT_K, kind="role", address=ADMIN, role="admin")
            put(3, ADMIN_K, kind="holder", plot="sinwood-7", address=HOLDER, name="Ana")
            put(4, HOLDER_K, kind="listing", plot="sinwood-7", deal="sale", price_usd="42000")
            put(5, ROOT_K, kind="approve", listing=str(uuid.UUID(int=4)))
            st, errs = acts.fold(d, ROOT)
            self.assertEqual(len(errs), 1); self.assertIn("01.md", errs[0])
            self.assertEqual(len(acts.public(st)["listings"]), 1)
            # a file whose body was edited after signing no longer recovers to its filed signer
            p = Path(d, "04.md"); p.write_text(p.read_text().replace("42000", "1"))
            st, errs = acts.fold(d, ROOT)
            self.assertTrue(any("04.md" in e for e in errs))
            self.assertEqual(acts.public(st)["listings"], [])
            r = subprocess.run([sys.executable, str(Path(__file__).parent / "acts.py"), "verify", d, ROOT], capture_output=True, text=True)
            self.assertEqual(r.returncode, 1)


class GateTest(unittest.TestCase):
    def test_acts_of_one_second_fold_in_the_order_they_were_taken(self):
        # the bench found it: sorted by signed time and id, a holder's binding could fold before the admin who
        # signed it was named, and the gate served a market its own re-fold refused
        import gate, time
        with tempfile.TemporaryDirectory() as d:
            subprocess.run(["git", "-C", d, "init", "-q", "-b", "main"], check=True)
            g = gate.Gate(d, ROOT, b"secret", None, push=False)
            at = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
            # ids chosen so that the later act sorts first
            for n, (key, f) in enumerate([(ROOT_K, dict(kind="role", address=ADMIN, role="admin")),
                                          (ADMIN_K, dict(kind="holder", plot="sinwood-7", address=HOLDER, name="Ana")),
                                          (HOLDER_K, dict(kind="listing", plot="sinwood-7", deal="sale", price_usd="42000"))]):
                message = acts.render({"id": str(uuid.UUID(int=1000 - n)), "at": at, **f})
                g.act(message, sign.sign(message, key))
            self.assertEqual(g.errors, [])
            self.assertEqual(len(g.state["listings"]), 1)
            st, errs = acts.fold(Path(d) / "acts", ROOT)
            self.assertEqual(errs, [])
            names = sorted(p.name for p in (Path(d) / "acts").iterdir())
            self.assertEqual([n.split("-")[1] for n in names], ["role", "holder", "listing"])

    def test_the_gate_refuses_an_old_act_and_a_second_use_of_an_id(self):
        import gate, time
        with tempfile.TemporaryDirectory() as d:
            subprocess.run(["git", "-C", d, "init", "-q", "-b", "main"], check=True)
            g = gate.Gate(d, ROOT, b"secret", None, push=False)
            old = acts.render({"kind": "profile", "id": str(uuid.UUID(int=1)), "at": "2020-01-01T00:00:00Z", "name": "x"})
            with self.assertRaises(ValueError): g.act(old, sign.sign(old, HOLDER_K))
            now = acts.render({"kind": "profile", "id": str(uuid.UUID(int=2)), "at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "name": "x"})
            g.act(now, sign.sign(now, HOLDER_K))
            with self.assertRaises(ValueError): g.act(now, sign.sign(now, HOLDER_K))


if __name__ == "__main__":
    unittest.main()
