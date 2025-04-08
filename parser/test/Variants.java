package ch.emilycares;
public enum Variants {
    A("a"),
    B("b"),
    C("c");

    private final String tag;
    private Variants(String tag) {
        this.tag = tag;
    }

    public String getTag() {
        return tag;
    }
}
