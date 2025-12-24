package ch.emilycares;

import java.util.Collection;
import java.util.List;

import java.util.stream.Stream;
import java.util.stream.StreamSupport;

public interface SuperInterface<E> extends Collection, List {
    default Stream<E> stream() {
        return StreamSupport.stream(spliterator(), false);
    }
}
