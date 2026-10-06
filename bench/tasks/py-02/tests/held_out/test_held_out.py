from baseline.solution import solve

def test_zero():
    assert solve(0) == 0

def test_seven():
    assert solve(7) == 49
