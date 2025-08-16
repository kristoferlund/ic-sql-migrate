import { useMutation } from "@tanstack/react-query";
import { backend } from "../../backend/declarations/index";

export default function useCreateDb() {
  return useMutation({
    mutationFn: () => {
      return backend.create();
    },
  });
}
