package a.test;
import java.util.logging.Logger;
import java.util.List;
import java.util.Map;
import java.util.HashMap;
public class Types {
  Logger LOG = Logger.getLogger("Types");
  boolean IS_ACTIVE = true;
  byte one_byte = 0;
  int one_int = 0;
  short one_short = 0;
  long one_long = 111l;
  double one_double = 0.0d;
  float one_float = 1.11f;
  char one_char = 'a';
  String one_string = "hihi";
  List<String> one_list = List.of("haha");
  Map<Integer, String> one_map = new HashMap();
  public static void main(String[] args) { }
}

