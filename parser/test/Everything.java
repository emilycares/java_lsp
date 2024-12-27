package ch.emilycares;

public class Everything {

    public Everything() {
    }
    int noprop;
    public int publicproperty;
    private int privateproperty;

    void method() {
    }

    public void public_method() {
    }

    private void private_method() {
    }

    int out() {
        return 0;
    }

    /**
     * Documentation
     * @param a
     * @param b
     * @return
     */
    int add(int a, int b) {
        return a + b;
    }

    static int sadd(int a, int b) {
      return a + b;
    }
}
