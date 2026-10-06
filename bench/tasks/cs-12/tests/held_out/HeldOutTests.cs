using System;
public class HeldOutTests
{
    public static void Run() {
        if (Solution.Solve(0) != 0) throw new Exception("zero");
        if (Solution.Solve(7) != 49) throw new Exception("seven");
    }
}
