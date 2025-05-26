package ch.emilycares;

import java.util.function.IntFunction;
import java.util.stream.Stream;

public interface InterfaceBase {
    <U> Stream<U> mapToObj(IntFunction<? extends U> mapper);

    public static<A> A a(final A arg) {
        return argument;
    }
}
