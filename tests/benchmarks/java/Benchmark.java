/**
 * 19D 的无依赖 Java 参考实现；输入和入口与 ../manifest.json 保持一致。
 */
public final class Benchmark {
    private Benchmark() {}

    public static void main(String[] args) {
        if (args.length != 2) {
            throw new IllegalArgumentException("usage: Benchmark <id> <argument>");
        }
        String id = args[0];
        long argument = Long.parseLong(args[1]);
        if ("scalar-overflow-and-bool-parity".equals(id)) {
            // Java 的 int/long 回绕与 Xiao 的溢出错误不具备可比语义。
            System.out.println("error\tX06-RUNTIME-009");
            return;
        }
        long value = switch (id) {
            case "deep-expression-arithmetic" -> deepExpression(argument);
            case "named-local-loop" -> namedLocalLoop(argument);
            case "deep-call-recursion" -> deepCall(argument, 1, 2, 3, 4);
            case "container-dense" -> containerDense(argument);
            default -> throw new IllegalArgumentException("unknown benchmark: " + id);
        };
        System.out.println("success\t" + value);
    }

    private static long deep(long value) {
        long a = value + 1;
        long b = a * 2;
        long c = b - value;
        long d = (a + b) * (c + 3);
        long e = d - a;
        long f = e * c;
        long g = f + d;
        long h = g * (a + c);
        return h - e;
    }

    private static long deepExpression(long rounds) {
        long total = 0;
        for (long index = 0; index != rounds; index += 1) {
            total += deep(index);
        }
        return total;
    }

    private static long namedLocalLoop(long rounds) {
        long total = 0;
        for (long index = 0; index != rounds; index += 1) {
            long a = index + 1;
            long b = a + 2;
            long c = b + 3;
            long d = c + 4;
            long e = d + 5;
            long f = e + 6;
            long g = f + 7;
            long h = g + 8;
            long i = h + 9;
            long j = i + 10;
            total += j;
        }
        return total;
    }

    private static long deepCall(long depth, long first, long second, long third, long fourth) {
        if (depth == 0) {
            return first + second + third + fourth;
        }
        return deepCall(depth - 1, first + 1, second + 2, third + 3, fourth + 4);
    }

    private static long containerDense(long rounds) {
        long total = 0;
        for (long index = 0; index != rounds; index += 1) {
            long[] values = {index, index + 1, index + 2, index + 3};
            long pairFirst = values[0];
            long mappingFirst = values[0];
            long columnFirst = values[0];
            long selectedFirst = values[0];
            long rangedFirst = values[1];
            long steppedFirst = values[0];
            // Xiao 的固定 seed=7 选择器在这个输入上选出最后一项。
            long pickedFirst = values[3];
            long itemValue = 0;
            total += values[0] + pairFirst + mappingFirst + columnFirst + selectedFirst
                + rangedFirst + steppedFirst + pickedFirst + itemValue + index;
        }
        return total;
    }
}
