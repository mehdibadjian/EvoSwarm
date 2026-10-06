using System;
public class TrustedTests
{
    public static void Run() {
        if (Solution.Solve(2) != 4) throw new Exception("two");
        if (Solution.Solve(10) != 100) throw new Exception("ten");
    }
}
