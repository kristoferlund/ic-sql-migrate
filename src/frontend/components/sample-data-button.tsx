import { Button } from "@/components/ui/button";
import { Database } from "lucide-react";
import useCreatePerson from "@/hooks/use-create-person";
import useCreateDb from "@/hooks/use-create-db";
import { useState } from "react";

const samplePersons = [
  { name: "Alice Johnson", age: 28n },
  { name: "Bob Smith", age: 35n },
  { name: "Charlie Brown", age: 42n },
  { name: "Diana Prince", age: 31n },
  { name: "Eve Williams", age: 27n },
  { name: "Frank Miller", age: 45n },
  { name: "Grace Lee", age: 33n },
  { name: "Henry Davis", age: 29n },
];

export default function SampleDataButton() {
  const createPerson = useCreatePerson();
  const createDb = useCreateDb();
  const [isLoading, setIsLoading] = useState(false);
  const [message, setMessage] = useState("");

  const handleInsertSampleData = async () => {
    setIsLoading(true);
    setMessage("");

    try {
      // First ensure database is created
      await createDb.mutateAsync();

      // Insert sample persons
      let successCount = 0;
      for (const person of samplePersons) {
        try {
          await createPerson.mutateAsync(person);
          successCount++;
        } catch (error) {
          console.error(`Failed to insert ${person.name}:`, error);
        }
      }

      setMessage(`Successfully inserted ${successCount} sample persons!`);
      setTimeout(() => setMessage(""), 3000);
    } catch (error) {
      console.error("Failed to insert sample data:", error);
      setMessage("Failed to insert sample data");
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="flex flex-col gap-2">
      <Button
        onClick={handleInsertSampleData}
        variant="outline"
        className="gap-2"
        disabled={isLoading}
      >
        <Database className="h-4 w-4" />
        {isLoading ? "Inserting..." : "Insert Sample Data"}
      </Button>
      {message && (
        <p className="text-sm text-green-300">{message}</p>
      )}
    </div>
  );
}
