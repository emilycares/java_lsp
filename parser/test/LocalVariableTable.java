package ch.emilycares;
import java.util.*;
public class LocalVariableTable {

  private HashSet<String> a = new HashSet<>();

  public void hereIsCode() {
    HashMap<Integer, String> a = new HashMap<>();
    a.put(1, "");
  }
  public int hereIsCode(int a, int b) {
    int o = a + b;
    return o - 1;
  }
}
