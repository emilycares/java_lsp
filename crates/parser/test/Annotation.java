package ch.emilycares;
public @interface Annotation {
    int value() default 0;
    String text() default "Hello";
}
