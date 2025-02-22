package ch.emilycares;
public enum Variants {
    A("a"),
    B("b"),
    C("c");

    private String tag;
    ICAPMode(String tag) {
        this.tag = tag;
    }

    public String getTag() {
        return tag;
    }
}
