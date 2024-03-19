export class A {
    secondFunction(a: number, b: number): number {
        console.log(a, b);
        return a * b;
    }

    firstFunction(a: number, b: number): number {
        console.log(a, b);
        return a + b;
    }
}
