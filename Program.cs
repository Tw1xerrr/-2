namespace Lecture
{

    internal class Program
    {
        static void Main(string[] args)
        {
            unsafe
            {
                int number = 86;
                int* ptr = &number; // указатель на переменную
                /*
                & - Взять адрес ячейки ОЗУ
                * - Подставить адрес взятый &
                ptr - Указатель (переменная для адреса)
                */

                Console.WriteLine($"Значение : {number}");
                Console.WriteLine($"Адрес ячейки: {(ulong)ptr:X} ");
                Console.WriteLine($"Значение через указатель: {*ptr}");

                *ptr = 100; // Разыменование или получение доступа к значению
                Console.WriteLine($"Новое значение переменной :{number}");

                int** ptr2 = &ptr; // Указатель на указатель
                Console.WriteLine($"Указатель на указатель : {**ptr2}");
                **ptr2 = 67;

                Console.WriteLine("$Исходная переменная {number}");

                int[] array = {1, 2, 3, 4, 5};

                fixed (int* arrayptr = array)
                {
                    for (int i = 0; i < array.Length; i++)
                    {
                        Console.WriteLine($"Элемент {i}: *(p + {i}) = {*arrayptr + i}");
                        *(arrayptr + i) *= 2; // *(Указатель + индекс) = массив[индекс]
                    }
                    Console.WriteLine("Массив после изменений:");
                    Console.WriteLine(string.Join(",", array));
                }


            }
        } 
        }
    }     