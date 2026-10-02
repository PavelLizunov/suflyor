"""Reference interval bookkeeping only, not reviewed line/semantic certification."""
import json
import hashlib
import unittest
from pathlib import Path
import coverage_gaps

ROOT=Path(__file__).resolve().parents[3]


class NavigationGapFixtures(unittest.TestCase):
    def test_merge_overlap_duplicates_adjacent_and_order(self):
        self.assertEqual(coverage_gaps.merge_ranges([(8,9),(1,3),(3,5),(1,3),(6,6)],10),[(1,6),(8,9)])

    def test_gaps_and_empty_full_intervals(self):
        self.assertEqual(coverage_gaps.gaps([(1,6),(8,9)],10),[(7,7),(10,10)])
        self.assertEqual(coverage_gaps.gaps([],5),[(1,5)])
        self.assertEqual(coverage_gaps.gaps([(1,5)],5),[])
        self.assertEqual(coverage_gaps.gaps([],0),[])

    def test_invalid_reference_ranges_fail_closed(self):
        for row in ((0,2),(2,1),(1,6),('1',2),(True,2)):
            with self.subTest(row=row):
                with self.assertRaises(ValueError):coverage_gaps.merge_ranges([row],5)

    def test_saved_ledger_no_semantic_credit_counts_and_hashes(self):
        p=ROOT/'docs/agent-map/reconciliation/navigation-gaps.json'
        v=json.loads(p.read_text())
        self.assertEqual(coverage_gaps.validate(ROOT,v),[])
        self.assertFalse(v['full_project_coverage'])
        self.assertEqual(v['summary']['semantically_reviewed_line_count'],'not_established')
        self.assertEqual(len(v['files']),298)
        self.assertEqual(v['summary']['files_with_some_precise_navigation']+v['summary']['files_without_precise_navigation'],298)
        for row in v['nonrange_pointers']:
            self.assertEqual(row['reason'],'directory_or_unranged_pointer_no_line_credit')


if __name__=='__main__':unittest.main()
