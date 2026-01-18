package ch.emilycares;
public class Syntax {
    public static void main(String[] args) {
        if (a ) {
        }
        if (true)
          1;

        if (a) {
        } else {
        }
        while (true) {
        }
        for(int i=1;i<=5;i++) {
        }
        outer: for(int i=1;i<=5;i++) {
        }
        String[] cars = {"Volvo", "BMW", "Ford", "Mazda"};
        System.out.println(cars[0]);
        int[][] myNumbers = { {1, 2, 3, 4}, {5, 6, 7} };
        myNumbers[1][2] = 9;
        for (String i : cars) {
          System.out.println(i);
        }
        double myDouble = 9.78d;
        int myInt = (int) myDouble;
        int a = a + b - c * d / e % f; 
        ++x;
        --x;
        x > y;
        y < x;
        x == y;
        x != y;
        x >= y;
        x <= y
        !(x < y);
        true && true || false;
        !true;
        !(true || false);
        String firstName = "John";
        String lastName = "Doe";
        System.out.println(firstName + " " + lastName);
        (time < 18) ? "Good day." : "Good evening.";
        switch(expression) {
          case x:
            // code block
            break;
          case y:
            // code block
            break;
          default:
            // code block
        }
        do {
          // code block to be executed
        }
        while (condition);
        break;
        continue;
        try {
          //  Block of code to try
        }
        catch(Exception e) {
          //  Block of code to handle errors
        }
        try {
          int[] myNumbers = {1, 2, 3};
          System.out.println(myNumbers[10]);
        } catch (Exception e) {
          System.out.println("Something went wrong.");
        } finally {
          System.out.println("The 'try catch' is finished.");
        }
        try {
            String some3 = "s";
        } catch (Exception | IOException e3) {
            String other3 = "o";
        } catch (IOException e3) {
            String other3 = "o";
        } finally {
            String fin3 = "a";
        }
        try {
            scanner = new Scanner(new File("test.txt"));
            while (scanner.hasNext()) {
                System.out.println(scanner.nextLine());
            }
        } catch (FileNotFoundException e) {
            e.printStackTrace();
        } finally {
            if (scanner != null) {
                scanner.close();
            }
        }
        try (AutoCloseableResourcesFirst af = new AutoCloseableResourcesFirst();
            AutoCloseableResourcesSecond as = new AutoCloseableResourcesSecond()) {

            af.doSomething();
            as.doSomething();
        }
        throw new ArithmeticException("Access denied");
        numbers.forEach( (n) -> { System.out.println(n); } );
        String message = switch (number) {
            case ONE -> {
                yield "Got a 1";
            }
            case TWO -> {
                yield "Got a 2";
            }
            case THREE, FOUR -> {
                yield "More than 2";
            }
        };

        Age oj1 = new Age()  {
            @Override
            public void getAge() 
            {
                System.out.print("Age is " + x);
            }
        };
        Arrays.stream(names).forEach(Geeks::print);
        """
        Java is better
        --Thorben""";
        return;
    }
}
class OuterClass {
  int x = 10;

  class InnerClass {
    int y = 5;
  }
}
abstract class Animal {
  public abstract void animalSound();
  public void sleep() {
    System.out.println("Zzz");
  }
}

